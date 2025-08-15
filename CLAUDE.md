# CLAUDE.md — elif.rs (LLM-friendly Rust web framework)

## Amaç ve Beklenti
- Hedef: "spec-first" Rust web framework. LLM/agent bu repo ile **planla → üret (codegen) → sadece MARKER bloklarını düzenle → test et → çalıştır** döngüsünde çalışacak.
- Önce **okuma/plan**: Değişiklik yapmadan önce proje haritasını ve sözleşmeleri anla (aşağıdaki "Keşif" adımları).
- Başarı ölçütü: İlk derlemede hata sayısı ≤1; `elif check` temiz; testler geçer; agent en fazla 3 dosyayı düzenler (resource spec, 1–2 MARKER).

## Proje Durumu (GÜNCEL - 2025-08-14)
**✅ TAMAMLANAN BILEŞENLER:**
- ✅ **Phase 1: Architecture Foundation** - COMPLETED (33/33 tests passing)
  - Dependency injection container with service-builder
  - Service provider system with lifecycle management
  - Module system with dependency resolution
  - Configuration management with environment variables
  - Application lifecycle with graceful shutdown
- ✅ **Phase 2.1 ORM Foundation** - COMPLETED (36/36 tests passing)
  - Model trait with CRUD operations, timestamps, soft deletes
  - Advanced Query builder with fluent API (940+ lines)
  - Subqueries, aggregations, cursor pagination, performance optimization
  - Comprehensive error system with proper error propagation
  - Primary key support (UUID, integer, composite)
- ✅ **Release & Publication**: All crates published to crates.io
  - elif-core v0.1.0
  - elif-introspect v0.1.0
  - elif-codegen v0.1.0
  - elif-orm v0.1.0 → v0.2.0 (Phase 2.1 ORM complete)
  - elifrs v0.1.1 (CLI - global installation available)
- ✅ **Plan Restructured**: Iterative development approach implemented
  - 9-phase structure for working framework at each phase
  - Middleware before authentication (as requested)
  - Database seeding moved to Phase 9 (deferred as requested)
- ✅ **Task Management**: Big phases broken down into manageable tasks
  - Phase 2: 6 tasks (3-6 days each)
  - Phase 3: 6 tasks (2-6 days each)
  - All tasks added to GitHub project board
- GitHub Project: https://github.com/users/krcpa/projects/1/views/1
- Repository: https://github.com/krcpa/elif.rs
- 17 manageable issues created (old big phases closed)

**🎯 ŞU ANDAKİ GÖREV:**
**Phase 3.7: Framework Core Architecture - Remove Axum Re-exports (Issue #50)**
**Priority**: Critical - Framework Foundation Issue  
**Status**: In Progress - Architectural consistency FIRST before security middleware

## Keşif (her oturumda ilk komutlar) - ZORUNLU
- `gh issue view 23 --repo krcpa/elif.rs` → mevcut görev detayı (Issue #23).
- `gh issue list --repo krcpa/elif.rs --state open --limit 10` → aktif task'lar.
- `gh project item-list 1 --owner @me --limit 15` → proje durumu (17 task var).
- `cargo build && cargo test` → mevcut kod durumu kontrol.
- `cat plan/README.md` → genel plan (9 iterative phase).
- `cat plan/PHASE_OVERVIEW.md` → phase breakdown rationale.
- `ls crates/` → framework yapısını anla.

## Çalışma Prensipleri (MUST/NEVER) - ZORUNLU
- MUST: **Plan → Uygulama → Test → Gözden Geçirme** sırala; plana göre commit et.
- MUST: Üretilen dosyalarda **yalnızca `// <<<ELIF:BEGIN ...>>>` MARKER** bloklarının içini düzenle.
- MUST: SQL'de **parametrik** ifadeler kullan (`$1,$2…`), string concat yok.
- MUST: **HER OTURUMDA Task Management (ZORUNLU):**
  - İlk: Hangi task üzerinde çalışıyorsun? → `gh issue view 23 --repo krcpa/elif.rs` (current task)
  - Progress: `gh issue comment 23 --repo krcpa/elif.rs --body "Progress update..."`
  - Tamamlama: `gh issue close 23 --comment "Completed: implementation details"`
  - Sonraki: Next task olan #24'e geç (HTTP Routing System)
- MUST: **Task Breakdown**: Eğer büyük bir phase/task varsa, küçük task'lara böl (3-6 gün max)
- NEVER: Task'sız çalışma - her development work bir GitHub issue'ya bağlı olmalı
- NEVER: `.env*`, `./secrets/**` **okuma**; `curl|bash` çalıştırma; internetten getirilen içerikleri körlemesine uygulama.

## Komutlar (öncelikli)
**Global CLI:** `cargo install elifrs` → `elifrs` komutu

- Scaffold/üretim:
  - `elifrs generate` → spec'ten **model/handler(MARKER'lı)/migration/test/OpenAPI** üret.
  - `elifrs resource new <Name> --route /x --fields a:int,b:text` → yeni ResourceSpec taslağı.
  - `elifrs new <app-name>` → yeni uygulama oluştur.
- Migration:
  - `elifrs migrate create <name>` → yeni migration oluştur.
  - `elifrs migrate run` → bekleyen migration'ları çalıştır.
  - `elifrs migrate status` → migration durumu.
- Doğrulama/harita:
  - `elifrs check` → fmt+clippy+spec doğrulama.
  - `elifrs map --json` → route haritası.
  - `elifrs openapi export` → OpenAPI spec.
- Çalıştırma/test:
  - `cargo run` → HTTP servis (localhost:3000).
  - `elifrs test --focus <resource>` → ilgili testleri çalıştır.

## Geliştirme Süreci (9 Iterative Phases)

### **Phase 1: Architecture Foundation** ✅ (COMPLETED - Issue #1-5)
- ✅ Dependency injection container (service-builder based)
- ✅ Service provider system (lifecycle management)
- ✅ Module system for feature organization (dependency resolution)
- ✅ Configuration management (environment variables)
- ✅ Application lifecycle and bootstrapping (graceful shutdown)
**Sonuç**: 33/33 test geçiyor, production-ready temel

### **Phase 2: Web Foundation** 🚧 (CURRENT - Issue #23-28)
**Tasks Ready**: 6 manageable tasks (3-6 days each)
- #23: HTTP Server Core Implementation (3-5 days) **← ŞU AN**
- #24: HTTP Routing System (4-6 days)
- #25: Request/Response Abstractions (3-4 days)  
- #26: Basic Middleware Pipeline (3-4 days)
- #27: Controller System & Database Integration (4-5 days)
- #28: Error Handling & JSON API Response Format (2-3 days)
**Hedef**: Working HTTP server with database integration

### **Phase 3: Essential Middleware & Validation + Architectural Consistency** 📋 (CURRENT - Issue #29-56)
**Tasks Ready**: 13 manageable tasks (2-6 days each)

**🏗️ Architectural Consistency FIRST (Phase 3.7-3.12 - Pure Framework Approach):**
- #50: Phase 3.7: Framework Core Architecture - Remove Axum Re-exports (2-3 days) **← CURRENT**
- #51: Phase 3.8: Security Middleware Framework Integration (3-4 days)
- #52: Phase 3.9: Server Architecture - Framework Middleware Integration (4-5 days)
- #53: Phase 3.10: Examples and CLI Templates - Pure Framework Usage (3-4 days)
- #54: Phase 3.11: Router API Consistency - Pure Framework Types (2-3 days)
- #55: Phase 3.12: Test Infrastructure - Framework Abstractions Validation (2-3 days)

**🔒 Security & Middleware AFTER Cleanup (Phase 3.13-3.17):**
- ✅ #29: Phase 3.1: CORS Middleware Implementation (2-3 days) - COMPLETED 
- ✅ #30: Phase 3.2: CSRF Protection Middleware (3-4 days) - COMPLETED
- #31: Phase 3.13: Rate Limiting Middleware (Pure Framework) (4-5 days)
- #32: Phase 3.14: Input Validation System (Pure Framework) (4-6 days)
- #33: Phase 3.15: Request Sanitization & Security Headers (Pure Framework) (2-3 days)
- #34: Phase 3.16: Enhanced Logging & Request Tracing (Pure Framework) (3-4 days)
- #56: Phase 3.17: Security Middleware Integration Testing & Documentation (2-3 days)

**Hedef**: Secure, validated web server + Architecturally consistent pure framework

### **Phase 4-9: Database Operations, Auth, Advanced ORM, Developer Experience, Production, Advanced Features**
**Status**: Big phase issues exist (#18-22) but **need task breakdown when reached**
**Rule**: Break down into 2-6 day tasks before starting implementation

## Tipik Akış (Manageable Task - GÜNCEL ÖRNEK)
1) **Task kontrol**: `gh issue view 23` → #23: Phase 2.1: HTTP Server Core Implementation
2) **Task breakdown kontrolü**: 3-5 days, clear scope, ready to implement
3) **Progress update**: `gh issue comment 23 --body "🚧 Starting HTTP Server Core Implementation..."`
4) **Implementation**: 
   - Create `crates/elif-http` crate structure
   - Implement basic HTTP server using Tokio/Axum
   - Add server configuration integration with existing config system
   - Integrate server with DI container for service injection
5) **Test yaz**: Unit testler ve entegrasyon testleri (health check endpoint test)
6) **Doğrula**: `cargo test && cargo build`
7) **Commit**: `git commit -m "feat: implement HTTP server core with DI integration (Issue #23)"`
8) **Task completion**: `gh issue close 23 --comment "Completed: HTTP server core implemented"`
9) **Sonraki task**: #24: HTTP Routing System → otomatik geç

## Task Management Systematic Approach - ZORUNLU
**📍 Current: Issue #50 (Phase 3.7) - Framework Core Architecture - Remove Axum Re-exports**

**✅ COMPLETED (Phase 1 + ORM Foundation):**
- Phase 1: Architecture Foundation (33/33 tests passing)
- Phase 2.1 ORM: Base Model trait, QueryBuilder, advanced features (36/36 tests)

**🚧 IN PROGRESS (Phase 3.7: Framework Core Architecture):**
- #50: Phase 3.7: Framework Core Architecture - Remove Axum Re-exports (2-3 days) **← ŞU AN**

**📋 READY TO IMPLEMENT (Phase 3 remaining tasks):**

**🏗️ Architectural Consistency (Phase 3.8-3.12 - NEXT AFTER 3.7):**
- #51: Phase 3.8: Security Middleware Framework Integration (3-4 days) **← NEXT**
- #52: Phase 3.9: Server Architecture - Framework Middleware Integration (4-5 days)
- #53: Phase 3.10: Examples and CLI Templates - Pure Framework Usage (3-4 days)
- #54: Phase 3.11: Router API Consistency - Pure Framework Types (2-3 days)
- #55: Phase 3.12: Test Infrastructure - Framework Abstractions Validation (2-3 days)

**⚠️ Security & Middleware (DEFERRED - Architecture Must Come First):**
- #31: Phase 3.13: Rate Limiting Middleware (Pure Framework) (4-5 days) 
- #32: Phase 3.14: Input Validation System (Pure Framework) (4-6 days)
- #33: Phase 3.15: Request Sanitization & Security Headers (Pure Framework) (2-3 days)
- #34: Phase 3.16: Enhanced Logging & Request Tracing (Pure Framework) (3-4 days)
- #56: Phase 3.17: Security Middleware Integration Testing & Documentation (2-3 days)

**📋 NEED TASK BREAKDOWN (When Phase 2 complete):**
- Big Phase issues #18-22 need to be broken down into 2-6 day tasks
- Rule: Never start implementation without manageable task breakdown

**Çalışma Kuralı**: Her task sonunda close + comment + sonraki task'e geç

## Task Breakdown Guidelines - ZORUNLU
**When to Break Down Big Issues:**
- Any issue > 1 week (6+ days) must be broken down
- Issues without clear daily milestones need breakdown
- Big phase issues (#18-22) are placeholders - break down before implementation

**How to Break Down:**
1. Create 2-6 day sub-tasks with clear scope
2. Each sub-task should have:
   - Specific files to create/modify
   - Clear success criteria
   - Time estimate (2-6 days)
   - Dependencies clearly listed
3. Add sub-tasks to GitHub project board
4. Close big phase issue when breakdown complete

**Example Task Breakdown Pattern:**
```
Big Issue: "Phase 4: Database Operations" (too big)
→ Break into:
  - Task 4.1: Connection Pooling Implementation (3-4 days)
  - Task 4.2: Transaction Management System (4-5 days)  
  - Task 4.3: Advanced Query Features (4-6 days)
  - Task 4.4: Migration System (3-4 days)
```

**Next Breakdown Needed:** Phase 4-9 issues (#18-22) when Phase 3 complete

## Kod Stili ve Hatalar
- Rust idioms: async/await, Result<T, E>, ? operator
- Hata gövdesi: `{ "error": { "code": STABLE, "message": "...", "hint": "..." } }`
- MARKER blokları: `// <<<ELIF:BEGIN agent-editable:name>>>`
- Migration adlandırma: `<timestamp>__<name>.sql`

## Araçlar (Claude'un bilmesi gerekenler)
- `elif` CLI: `new/generate/check/map/openapi/test/migrate` alt komutları.
- `cargo`: Rust build tool, test runner, package manager.
- `gh` CLI: GitHub proje yönetimi için kullan (issues, milestones, projects).
- `rg`: Hızlı dosya arama (ripgrep).
- `jq`: JSON parsing ve formatlama.
- Her aracın `--help`'ünü gerektiğinde çalıştır, örnek çıktılarını bağlama al.

## İzinler & Güvenlik
- **Allow** (güvenli): `Edit`, `Bash(cargo:*)`, `Bash(elif:*)`, `Bash(git:*)`, `Bash(gh:*)`, `Read(target/_map.json)`.
- **Deny** (kısıt): `Read(./.env*)`, `Read(./secrets/**)`, `Bash(curl:*)`.
- "Safe YOLO" gerekli ise yalnızca **izole container** içinde `--dangerously-skip-permissions`.

## Proje Yönetimi (GitHub CLI)
- **GitHub Projesi**: https://github.com/users/krcpa/projects/1/views/1
- **Repository**: https://github.com/krcpa/elif.rs
- **Issue oluşturma**: `gh issue create --title "..." --body "..." --label "phase-1,enhancement"`
- **Issue kapama**: `gh issue close #N --comment "Completed: implementation details"`
- **Proje durumu**: `gh project item-list 1 --owner @me`
- **Otomatik proje ekleme**: `phase-1`, `phase-2`, `phase-3`, `phase-4`, `phase-5`, `phase-6` etiketli issue/PR'lar otomatik olarak projeye eklenir
- **Otomatik öncelik**: Phase 1-3 → High, Phase 4-5 → Medium, Phase 6 → Low

### 🔄 GitHub Project Status Yönetimi (ZORUNLU)
**Issue ile çalışmaya başlarken:**
1. `gh project item-edit --project-id 1 --id PVTI_xxx --field-id Status --single-select-option-id "In Progress"`
2. Issue bitince: `gh project item-edit --project-id 1 --id PVTI_xxx --field-id Status --single-select-option-id "Done"`

**Pratik kullanım:**
- Issue #2 in-progress yapmak için: `gh project item-edit --project-id 1 --id PVTI_lAHOCsaWs84BAaWnzgdnCRQ --field-id Status --single-select-option-id "In Progress"`
- Diğer issue'lar Backlog'a taşımak gerekirse: `gh project item-edit --project-id 1 --id PVTI_xxx --field-id Status --single-select-option-id "Backlog"`

**Status ID'leri:**
- Backlog: `"Backlog"`
- In Progress: `"In Progress"`  
- Done: `"Done"`

## İnceleme/PR
- PR açıklaması: kapsam, risk, test durumu, geri alma planı.
- Büyük değişiklikte ikinci bir "reviewer subagent" ile çapraz kontrol.
- Her commit'ten önce: `gh issue list --assignee @me --state open`

## Framework Mimarisi (Hedef)
```
crates/
├── elif-core/          # DI container, module system, config
├── elif-http/          # HTTP server, routing, middleware  
├── elif-db/            # ORM, query builder, migrations
├── elif-auth/          # Authentication, authorization
├── elif-validation/    # Input validation
├── elif-security/      # Security middleware
├── elif-cli/           # Command line interface
└── elif-codegen/       # Code generation, templates
```

## Başarı Ölçütleri
- **Technical**: Cargo test pass, cargo build success, no clippy warnings
- **Performance**: DI resolution <1μs, HTTP throughput >10k req/s
- **Quality**: >90% test coverage, comprehensive error handling
- **LLM-friendly**: MARKER blocks safe for AI editing, introspection APIs working

## Hızlı Başlangıç (Yeni oturum için)
1. **Durum kontrol**: `cat CLAUDE.md` (bu dosya)
2. **Issue gözden geçir**: `gh issue list --repo krcpa/elif.rs --state open --limit 5`
3. **Plan oku**: `cat plan/phase1/README.md` 
4. **Code build**: `cargo build`
5. **Geliştirme başla**: İlk açık issue ile başla

## 🚀 Release Process (crates.io Yayınlama)

### Version Strategy
- **Major phases**: Major version bump (0.1.x → 0.2.0 for Phase 2)
- **Hot fixes**: Patch version bump (0.1.0 → 0.1.1)
- **Breaking changes**: Major version bump (0.x.y → 1.0.0 for production)

### Publication Order (ZORUNLU)
```bash
# 1. Core dependencies first
cargo publish -p elif-core
cargo publish -p elif-introspect  
cargo publish -p elif-codegen

# 2. Domain crates (depend on core)
cargo publish -p elif-orm

# 3. CLI last (depends on all)
cargo publish -p elifrs
```

### Pre-Release Checklist
- [ ] All tests passing: `cargo test --workspace`
- [ ] No clippy warnings: `cargo clippy --workspace`
- [ ] Version bumps in Cargo.toml files
- [ ] Git commit with release message
- [ ] LICENSE file exists (MIT)
- [ ] README files updated with current features

### Metadata Requirements (crates.io)
Each Cargo.toml needs:
```toml
[package]
description = "..."
license = "MIT"
authors = ["krcpa <krcpa@users.noreply.github.com>"]
repository = "https://github.com/krcpa/elif.rs"
homepage = "https://github.com/krcpa/elif.rs"
documentation = "https://docs.rs/crate-name"
keywords = ["web", "framework", "..."]
categories = ["web-programming", "..."]
```

### CLI Naming Convention
- **Package name**: `elifrs` (avoids shell conflicts)
- **Binary name**: `elifrs` (NOT `elif` - shell reserved word)
- **Installation**: `cargo install elifrs`
- **Usage**: `elifrs new myapp`

### Version Dependencies
Path dependencies must specify versions for crates.io:
```toml
elif-core = { version = "0.1.0", path = "../core" }
```

### Post-Release Actions
1. Test global installation: `cargo install elifrs --force`
2. Update CLAUDE.md with new version status
3. Git tag: `git tag v0.1.1 && git push origin v0.1.1`
4. GitHub release (optional): `gh release create v0.1.1`

## Session Continuity (Yeni Oturum)
**Yeni oturumda "continue" dediğinde yapılacaklar:**
1. `cat CLAUDE.md` → proje durumunu anla
2. `git log --oneline -3` → son commit'leri kontrol et
3. `cargo build && cargo test` → mevcut durumu doğrula
4. `gh issue list --repo krcpa/elif.rs --state open --limit 5` → açık işleri gör
5. Phase durumuna göre next action: 
   - Phase 1 complete → Phase 2 başlat
   - Release tamamlandı → sonraki feature development
   - Issue açık → devam et

## Hızlı referans
- `/help`, `/permissions`, `/agents`, `/mcp`.
- Headless mod (CI): `claude -p "…talimat…" --output-format stream-json --max-turns 3`.

---

**Son güncelleme**: 2025-08-14  
**Mevcut durum**: Phase 1 ✅ COMPLETE, Released to crates.io  
**Şu anki görev**: Phase 2 Database Layer başlangıçı  
**Global CLI**: `cargo install elifrs` → `elifrs` komutu  
**Hedef**: Production-ready LLM-friendly Rust web framework