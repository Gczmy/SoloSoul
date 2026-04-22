# Third Party Licenses

This document contains attribution notices for third-party software used in SoloSoul.

## License Overview

SoloSoul uses only **commercial-friendly open source licenses**. No GPL, AGPL, or LGPL dependencies detected.

| License Type | Risk Level | SoloSoul Status |
|--------------|------------|-----------------|
| MIT | ✅ Low | Used by core packages |
| Apache 2.0 | ✅ Low | Used by crypto library |
| BSD-3-Clause | ✅ Low | Used by multiple packages |
| BSD-2-Clause | ✅ Low | Safe for commercial use |

## Direct Dependencies

### Core Framework
| Package | Version | License | Commercial Use |
|---------|---------|---------|----------------|
| flutter | SDK | BSD-3-Clause | ✅ Yes |
| flutter_riverpod | 2.6.1 | MIT | ✅ Yes |
| go_router | 14.2.0 | MIT | ✅ Yes |

### Cryptography & Security
| Package | Version | License | Commercial Use |
|---------|---------|---------|----------------|
| cryptography | 2.9.0 | Apache 2.0 | ✅ Yes |
| pointycastle | 3.9.1 | MIT | ✅ Yes |
| encrypt | 5.0.3 | BSD-3-Clause | ✅ Yes |
| flutter_secure_storage | 9.2.2 | MIT | ✅ Yes |
| local_auth | 2.3.0 | BSD-3-Clause | ✅ Yes |

### UI & Rendering
| Package | Version | License | Commercial Use |
|---------|---------|---------|----------------|
| google_fonts | 6.2.1 | BSD-3-Clause | ✅ Yes |
| flutter_animate | 4.5.0 | MIT | ✅ Yes |
| flutter_markdown | 0.7.4 | BSD-3-Clause | ✅ Yes |

### Data & Serialization
| Package | Version | License | Commercial Use |
|---------|---------|---------|----------------|
| freezed | 2.5.7 | MIT | ✅ Yes |
| json_serializable | 6.8.0 | BSD-3-Clause | ✅ Yes |
| uuid | 4.4.0 | BSD-3-Clause | ✅ Yes |

### FFI & Native
| Package | Version | License | Commercial Use |
|---------|---------|---------|----------------|
| ffi | 2.1.0 | BSD-2-Clause | ✅ Yes |
| flutter_rust_bridge | 2.0.0 | BSD-2-Clause | ✅ Yes |

## License Summaries

### MIT License
Permissive license allowing commercial use, modification, distribution, and private use. Source code must include copyright notice.

### Apache 2.0
Similar to MIT but includes patent rights grant and explicit state that trademarks are not licensed.

### BSD-3-Clause
Permissive license allowing redistribution and commercial use. Cannot use contributor names for endorsement without permission.

### BSD-2-Clause
Simplified BSD license with only 2 clauses. Very permissive, suitable for commercial products.

## No Copyleft Dependencies

**Confirmed: No GPL, AGPL, or LGPL dependencies found.**

All dependencies use permissive licenses that allow:
- ✅ Commercial use
- ✅ Private use
- ✅ Modification
- ✅ Distribution
- ✅ Patent use
- ✅ Embedding in commercial products

## Attribution Requirements

When redistributing SoloSoul, include this notice:

```
SoloSoul uses the following open source packages:
- cryptography (Apache 2.0)
- flutter_riverpod (MIT)
- go_router (MIT)
- pointycastle (MIT)
- encrypt (BSD-3-Clause)
- google_fonts (BSD-3-Clause)
- flutter_secure_storage (MIT)
- local_auth (BSD-3-Clause)
- flutter_animate (MIT)
- flutter_markdown (BSD-3-Clause)
- freezed (MIT)
- json_serializable (BSD-3-Clause)
- uuid (BSD-3-Clause)
- ffi (BSD-2-Clause)
- flutter_rust_bridge (BSD-2-Clause)
```

## Verification

Dependencies verified via:
- pub.dev package pages
- Direct license documentation review
- No GPL/AGPL/LGPL patterns found in pubspec.lock

Last updated: 2026-04-22

---

**SoloSoul - Be the only master of your digital self.**
