use futures::stream::{self, StreamExt};
use pyo3::exceptions::{PyKeyboardInterrupt, PyRuntimeError};
use pyo3::prelude::*;
use reqwest::{Client, StatusCode};
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;
use tokio::fs;
use tokio::time::interval;

const LIKELY_404: &[&str] = &[
    "404", "not found", "nothing found", "page not found", "no encontrado",
    "index of /wp-content/plugins",
];
const GENERIC_TITLES: &[&str] = &[
    "page not found", "nothing here", "error", "wordpress &rsaquo; error",
];

const POINT_200_OK: u8 = 35;
const POINT_BODY_REAL: u8 = 30;
const POINT_ASSET_OK: u8 = 20;

const THRESHOLD_FOUND: u16 = 700;
const THRESHOLD_POSSIBLE: u16 = 400;

#[derive(Default)]
struct Evidence {
    points: u16,
    max_possible: u16,
}
impl Evidence {
    fn add(&mut self, pts: u8) {
        self.points += pts as u16;
        self.max_possible += pts as u16;
    }
    fn skip(&mut self, pts: u8) {
        self.max_possible += pts as u16;
    }
    fn percentage(&self) -> u16 {
        if self.max_possible == 0 {
            0
        } else {
            (self.points * 1000) / self.max_possible
        }
    }
}

#[derive(Debug)]
enum PluginState {
    Found,
    Possible,
    NotFound,
    Error(String),
}

#[pyclass]
#[derive(Clone)]
pub struct ScanResult {
    #[pyo3(get)]
    pub plugin: String,
    #[pyo3(get)]
    pub state: String,
}
#[pymethods]
impl ScanResult {
    fn __repr__(&self) -> PyResult<String> {
        Ok(format!("<ScanResult {}:{}>", self.plugin, self.state))
    }
}

#[pyclass]
struct PyProgress {
    callable: Py<PyAny>,
}
#[pymethods]
impl PyProgress {
    fn call(&self, py: Python<'_>, idx: usize, res: ScanResult) -> PyResult<()> {
        self.callable.call1(py, (idx, res))?;
        Ok(())
    }
}

#[pyclass]
struct Scanner {
    target: String,
    rate: u32,
    timeout: u64,
}
#[pymethods]
impl Scanner {
    #[new]
    #[pyo3(signature = (target, rate_per_sec, timeout_secs=15))]
    fn py_new(target: String, rate_per_sec: u32, timeout_secs: u64) -> Self {
        Self {
            target,
            rate: rate_per_sec,
            timeout: timeout_secs,
        }
    }

    fn scan(&self, wordlist: PathBuf, progress: Py<PyAny>) -> PyResult<Vec<ScanResult>> {
        let prog = Arc::new(PyProgress { callable: progress });
        let rt = tokio::runtime::Runtime::new()
            .map_err(|e| PyRuntimeError::new_err(e.to_string()))?;
        match rt.block_on(async { self.run_scan(wordlist, prog).await }) {
            Ok(v) => Ok(v),
            Err(e) => Err(PyKeyboardInterrupt::new_err(e.to_string())),
        }
    }
}

impl Scanner {
    async fn run_scan(
        &self,
        wordlist: PathBuf,
        on_prog: Arc<PyProgress>,
    ) -> Result<Vec<ScanResult>, pyo3::PyErr> {
        let workers = self.rate.clamp(1, 256) as usize;
        let period = Duration::from_secs_f64(1.0 / (self.rate as f64));
        let mut ticker = interval(period);
        ticker.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Burst);

        let client = Client::builder()
            .timeout(Duration::from_secs(self.timeout))
            .user_agent("Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36")
            .build()
            .map_err(|e| PyRuntimeError::new_err(e.to_string()))?;

        let plugins: Vec<String> = {
            let txt = fs::read_to_string(&wordlist)
                .await
                .map_err(|e| PyRuntimeError::new_err(e.to_string()))?;
            txt.lines()
                .map(str::trim)
                .filter(|l| !l.is_empty())
                .map(String::from)
                .collect()
        };

        let total = plugins.len();
        let (tx, rx) = tokio::sync::mpsc::unbounded_channel::<(usize, String)>();
        let producer = async move {
            for (idx, plugin) in plugins.into_iter().enumerate() {
                ticker.tick().await;
                tx.send((idx, plugin)).ok();
            }
        };
        tokio::spawn(producer);

        let mut results = Vec::with_capacity(total);
        let consumer = stream::unfold(rx, move |mut rx| {
            let cli = client.clone();
            let tgt = self.target.clone();
            let prog = on_prog.clone();
            async move {
                let (idx, plugin) = rx.recv().await?;
                let fut = async move {
                    let state = Self::check_plugin(&cli, &tgt, &plugin).await.ok()?;
                    let state_str = match &state {
                        PluginState::Found => "found",
                        PluginState::Possible => "possible",
                        PluginState::NotFound => "not_found",
                        PluginState::Error(e) => return Some(ScanResult {
                            plugin: plugin.clone(),
                            state: format!("error:{e}"),
                        }),
                    };
                    let res = ScanResult {
                        plugin: plugin.clone(),
                        state: state_str.into(),
                    };
                    Python::attach(|py| prog.call(py, idx, res.clone())).ok()?;
                    Some(res)
                };
                Some((fut, rx))
            }
        })
        .buffer_unordered(workers);

        let mut consumer = Box::pin(consumer);
        while let Some(Some(res)) = consumer.as_mut().next().await {
            results.push(res);
        }
        Ok(results)
    }

    async fn check_plugin(
        client: &Client,
        target: &str,
        plugin: &str,
    ) -> Result<PluginState, pyo3::PyErr> {
        let base = format!("{}/wp-content/plugins/{}/", target.trim_end_matches('/'), plugin);
        let resp = match client.get(&base).send().await {
            Ok(r) => r,
            Err(e) => return Ok(PluginState::Error(format!("{plugin}: {e}"))),
        };
        let status = resp.status();

        // 1. 403 → comprobar readme.txt SIEMPRE
        if status == StatusCode::FORBIDDEN {
            let readme_resp = client
                .head(format!("{base}readme.txt"))
                .send()
                .await;
            return if readme_resp.map(|r| r.status() == StatusCode::OK).unwrap_or(false) {
                Ok(PluginState::Found)
            } else {
                Ok(PluginState::Possible)
            };
        }

        // 2. Otros no-2xx → NotFound
        if !status.is_success() {
            return Ok(PluginState::NotFound);
        }

        // 3. Es 200 → seguir con validaciones adicionales
        let mut ev = Evidence::default();
        ev.add(POINT_200_OK);

        let body_bytes = resp.bytes().await.unwrap_or_default();
        let text = String::from_utf8_lossy(&body_bytes[..body_bytes.len().min(8192)]).to_lowercase();
        let looks_real = !LIKELY_404.iter().any(|pat| text.contains(pat))
            && !GENERIC_TITLES.iter().any(|g| text.contains(g));
        if looks_real {
            ev.add(POINT_BODY_REAL);
        } else {
            ev.skip(POINT_BODY_REAL);
        }

        let assets = ["readme.txt", "style.css", "assets/icon-128x128.png"];
        let mut any_asset = false;
        for asset in &assets {
            let asset_resp = client.head(format!("{base}{asset}")).send().await;
            if asset_resp.map(|r| r.status() == StatusCode::OK).unwrap_or(false) {
                any_asset = true;
                break;
            }
        }
        if any_asset {
            ev.add(POINT_ASSET_OK);
        } else {
            ev.skip(POINT_ASSET_OK);
        }

        let score = ev.percentage();
        Ok(match score {
            s if s >= THRESHOLD_FOUND => PluginState::Found,
            s if s >= THRESHOLD_POSSIBLE => PluginState::Possible,
            _ => PluginState::NotFound,
        })
    }
}

#[pymodule]
fn plugins_ext(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_class::<Scanner>()?;
    m.add_class::<ScanResult>()?;
    Ok(())
}
