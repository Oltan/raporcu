# common/src/lib.rs

**Amaç:** raporcu ve stajci'nin paylaştığı sürüm kuralı. `allure` ve `api` modüllerini dışa açar.

## Sabitler
| Ad | Değer | Anlam |
|----|-------|-------|
| `VERSION` | paket sürümü (`Cargo.toml` → `0.1.0`) | stajci ve sunucu aynı derlemeden gelir |
| `VERSION_HEADER` | `x-stajci-version` | stajci her CI isteğinde bu başlıkta kendi sürümünü gönderir (SW-10.9) |

## `versions_compatible(a, b) -> bool`
İki sürüm dizesinin **major ve minor** parçaları aynıysa uyumlu. `"0.1.0"` ~ `"0.1.7"` uyumlu,
`"0.2.0"` değil. Boş ya da noktasız dize uyumsuz. Patch farkı yok sayılır.

## Testler
`test_sw_10_9_version_compat_rule`

## Python karşılığı
```python
VERSION = "0.1.0"; VERSION_HEADER = "x-stajci-version"
def versions_compatible(a, b):
    pa, pb = a.strip().split("."), b.strip().split(".")
    return len(pa) >= 2 and len(pb) >= 2 and pa[:2] == pb[:2]
```
