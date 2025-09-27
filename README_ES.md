# 🚀 plugins-ext  
**Descubrimiento de plugins para WordPress de alto rendimiento, escrito en Rust y listo para Python.**  

[![Rust](https://img.shields.io/badge/rust-1.70+-orange.svg)](https://rust-lang.org)  
[![PyO3](https://img.shields.io/badge/powered%20by-PyO3-0096FF.svg)](https://pyo3.rs)  

> 🔌 Plugin oficial con **límite de tasa, asíncrono y bajos falsos positivos** para [WPAT](https://github.com/WPAT-Project/WPAT) (WordPress Professional Audit Tool).

---

## ✨ ¿Qué es?

`plugins-ext` es una **extensión nativa en Rust** que acelera la enumeración de plugins de WordPress:

* ⚡ **Extremadamente rápido** – asíncrono, concurrente y sin bloqueos  
* 🎯 **Preciso** – filtrado inteligente de errores 404 / títulos genéricos → mínimos falsos positivos  
* 🐍 **Python-friendly** – importación directa, callbacks de progreso, compatible con `asyncio`  
* 🚦 **Educado** – gobernador de solicitudes por segundo mantiene contento al objetivo (y a tu ISP)  
* 🔐 **Seguro** – solo TLS, timeout configurable, sin fugas / sin `unsafe`

---

## 🏁 Inicio rápido

1. **Instalar** (el wheel llegará pronto – por ahora compila desde el código fuente)  
   ```bash
   # (1) obtener Rust estable
   curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
   # (2) clonar
   git clone https://github.com/WPAT-Project/plugins-ext && cd plugins-ext
   # (3) compilar e instalar el wheel de Python
   pip install maturin
   maturin develop --release
   ```

2. **Enumerar**  
   ```python
   from plugins_ext import Scanner

   def live(feed, res):
       print(f"{feed:>4}  ➜  {res.plugin:<30} {res.state}")

   scanner = Scanner("https://example.com", rate_per_sec=40, timeout_secs=12)
   results = scanner.scan("wordlist/top-6000.txt", live)

   found = [r.plugin for r in results if r.state == "found"]
   print(f"\n✅  {len(found)} plugins confirmados")
   ```

---

## 🧠 ¿Cómo funciona?

| Etapa | Tecnología | Descripción |
|-------|------------|-------------|
| **Ingesta de lista de palabras** | `tokio::fs` | Streaming asíncrono, recorte sin copia |
| **Gobernador de tasa** | `tokio::time::Interval` | Resistente a ráfagas, RPS exacto |
| **Motor HTTP** | `reqwest` + `rustls-tls` | HTTP/2, keep-alive, bajo consumo de memoria |
| **Heurística 404** | Patrones sin regex | Más de 25 marcadores de error genéricos + verificación de títulos |
| **Confirmación** | HEAD múltiple de activos | `readme.txt` ⬄ `style.css` ⬄ `icon-128x128.png` |
| **Puente Python** | `PyO3` | Seguro con GIL, callbacks `Py<PyAny>`, sin copias |

---

## ⚙️ Referencia de la API

### `Scanner(target, rate_per_sec=30, timeout_secs=15)`

| Parámetro | Tipo | Por defecto | Notas |
|-----------|------|-------------|-------|
| `target` | `str` | — | URL base del sitio WordPress (`https://foo.com`) |
| `rate_per_sec` | `int` | `30` | Solicitudes por segundo (limitado 1-256) |
| `timeout_secs` | `int` | `15` | Timeout de socket por solicitud |

### `scan(wordlist, progress=None) -> list[ScanResult]`

* `wordlist`: tipo ruta (`str`, `pathlib.Path`) archivo de texto con un slug de plugin por línea  
* `progress`: callable opcional `f(index: int, result: ScanResult) -> None` invocado al completar cada prueba  
* Retorna: `list[ScanResult]` (el orden ≠ orden de entrada – usa `.plugin` para correlacionar)

### `ScanResult`

| Atributo | Tipo | Valor |
|----------|------|-------|
| `plugin` | `str` | Slug probado |
| `state`  | `str` | `found` \| `possible` \| `not_found` \| `error:<msg>` |

---

## 🧪 Ejemplo de salida

```
   0  ➜  akismet                      found
   1  ➜  jetpack                      found
   2  ➜  wordfence                    possible
   3  ➜  fake-plugin-xyz              not_found
...
✅  312 plugins confirmados
```

---

## 🧩 Integración con WPAT

`plugins-ext` se incluye como un **plugin de primera clase** dentro de [WPAT](https://github.com/WPAT-Project/WPAT).  

## 📊 Rendimiento

| Hardware | Lista de palabras | Tasa | Tiempo | RAM |
|----------|-------------------|------|--------|-----|
| MBP M2   | 10 k              | 200 rps | 50 s | ≈ 35 MB |
| VPS 8 vCPU | 50 k            | 500 rps | 100 s | ≈ 90 MB |

*(El rendimiento real depende de la latencia de red y del tiempo de respuesta del objetivo.)*

---

<div align="center">

**⭐ Da una estrella** al repo si te fue útil

</div>
