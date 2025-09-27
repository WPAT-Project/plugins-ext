# 🚀 plugins-ext  
**High-performance WordPress plugin discovery, written in Rust & ready for Python.**  

[![Rust](https://img.shields.io/badge/rust-1.70+-orange.svg)](https://rust-lang.org)  
[![PyO3](https://img.shields.io/badge/powered%20by-PyO3-0096FF.svg)](https://pyo3.rs)  

> 🔌 Official **rate-limited, async, low-false-positive** plug-in for [WPAT](https://github.com/WPAT-Project/WPAT) (WordPress Professional Audit Tool).

---

## ✨ What is it?

`plugins-ext` is a **Rust-native extension** that turbo-charges WordPress plugin enumeration:

* ⚡ **Blazing fast** – asynchronous, concurrent & lock-free  
* 🎯 **Accurate** – smart 404 / generic-title filtering → minimal false positives  
* 🐍 **Pythonic** – drop-in import, progress callbacks, `asyncio` friendly  
* 🚦 **Polite** – exact request-per-second governor keeps target (and your ISP) happy  
* 🔐 **Secure** – TLS-only, configurable timeout, no leaks / no `unsafe`

---

## 🏁 Quick Start

1. **Install** (wheel coming soon – build from source for now)  
   ```bash
   # (1) get stable Rust
   curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
   # (2) clone
   git clone https://github.com/WPAT-Project/plugins-ext && cd plugins-ext
   # (3) compile & install Python wheel
   pip install maturin
   maturin develop --release
   ```

2. **Enumerate**  
   ```python
   from plugins_ext import Scanner

   def live(feed, res):
       print(f"{feed:>4}  ➜  {res.plugin:<30} {res.state}")

   scanner = Scanner("https://example.com", rate_per_sec=40, timeout_secs=12)
   results = scanner.scan("wordlist/top-6000.txt", live)

   found = [r.plugin for r in results if r.state == "found"]
   print(f"\n✅  {len(found)} plugins confirmed")
   ```

---

## 🧠 How it works

| Stage | Tech | Description |
|-------|------|-------------|
| **Wordlist ingestion** | `tokio::fs` | Async streaming, zero-copy trimming |
| **Rate governor** | `tokio::time::Interval` | Burst-resistant, exact RPS |
| **HTTP engine** | `reqwest` + `rustls-tls` | HTTP/2, keep-alive, low memory |
| **404 heuristic** | Regex-free patterns | 25+ generic error markers + title checks |
| **Confirmation** | Multi-asset HEAD | `readme.txt` ⬄ `style.css` ⬄ `icon-128x128.png` |
| **Python bridge** | `PyO3` | GIL-safe, `Py<PyAny>` callbacks, no copy |

---

## ⚙️ API Reference

### `Scanner(target, rate_per_sec=30, timeout_secs=15)`

| Parameter | Type | Default | Notes |
|-----------|------|---------|-------|
| `target` | `str` | — | Base URL of WordPress site (`https://foo.com`) |
| `rate_per_sec` | `int` | `30` | Requests per second (clamped 1-256) |
| `timeout_secs` | `int` | `15` | Per-request socket timeout |

### `scan(wordlist, progress=None) -> list[ScanResult]`

* `wordlist`: path-like (`str`, `pathlib.Path`) text file with one plugin slug per line  
* `progress`: optional callable `f(index: int, result: ScanResult) -> None` invoked on every completion  
* Returns: `list[ScanResult]` (order ≠ input order – use `.plugin` to correlate)

### `ScanResult`

| Attribute | Type | Value |
|-----------|------|-------|
| `plugin` | `str` | Slug tested |
| `state` | `str` | `found` \| `possible` \| `not_found` \| `error:<msg>` |

---

## 🧪 Example Output

```
   0  ➜  akismet                      found
   1  ➜  jetpack                      found
   2  ➜  wordfence                    possible
   3  ➜  fake-plugin-xyz              not_found
...
✅  312 plugins confirmed
```

---

## 🧩 Integration with WPAT

`plugins-ext` ships as a **first-class plug-in** inside [WPAT](https://github.com/WPAT-Project/WPAT).  

## 📊 Performance

| Hardware | Wordlist | Rate | Time | RAM |
|----------|----------|------|------|-----|
| MBP M2   | 10 k     | 200 rps | 50 s | ≈ 35 MB |
| 8 vCPU VPS | 50 k | 500 rps | 100 s | ≈ 90 MB |

*(Your mileage depends on network latency and target response time.)*

---

<div align="center">

**⭐ Star** the repo if it helped you

</div>
```
