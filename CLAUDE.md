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
- ✅ **Phase 2: Web Foundation** - COMPLETED (112/112 tests passing)
  - HTTP server and routing system with pure framework types
  - Basic middleware pipeline architecture
  - Request/response handling & JSON API abstractions  
  - Controller system with database integration
  - Production-ready web server foundation
- ✅ **Phase 2.1 ORM Foundation** - COMPLETED (39/39 tests passing)
  - Model trait with CRUD operations, timestamps, soft deletes
  - Advanced Query builder with fluent API (940+ lines)
  - Subqueries, aggregations, cursor pagination, performance optimization
  - Comprehensive error system with proper error propagation
  - Primary key support (UUID, integer, composite)
- ✅ **Phase 3: Security & Validation** - COMPLETED (151/151 tests passing)
  - Security middleware (CORS, CSRF, rate limiting)
  - Input validation and sanitization system
  - Logging and request tracing middleware
  - Security headers and protection
  - Production-ready security infrastructure
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
**Phase 4: Database Operations Foundation (Issues #60-65)**
**Priority**: High - Core Database Infrastructure  
**Status**: Ready to Start - Phase 3 Complete, Database Abstraction Needed

**Current Phase 4 Tasks:**
- **#60**: Phase 4.1: Database Service Integration (3 days)
- **#61**: Phase 4.2: Basic Connection Pool Management (3 days)  
- **#62**: Phase 4.3: Simple Transaction Support (4 days)
- **#63**: Phase 4.4: Basic Migration System (4 days)
- **#64**: Phase 4.5: Model-Database Integration (3 days)
- **#65**: Phase 4.6: Basic CRUD Operations (2 days)

**Goal**: Complete foundational database layer with DI integration, basic pooling, transactions, migrations, and working CRUD operations.

## Keşif (her oturumda ilk komutlar) - ZORUNLU
- `gh issue list --repo krcpa/elif.rs --state open --limit 5` → açık task'ları kontrol et.
- `gh issue list --repo krcpa/elif.rs --state open --limit 10` → aktif task'lar.
- `gh project item-list 1 --owner @me --limit 15` → proje durumu.
- `cargo build && cargo test` → mevcut kod durumu kontrol.
- `cat plan/README.md` → genel plan (9 iterative phase).
- `cat plan/PHASE_OVERVIEW.md` → phase breakdown rationale.
- `ls crates/` → framework yapısını anla.

## Çalışma Prensipleri (MUST/NEVER) - ZORUNLU
- MUST: **Plan → Uygulama → Test → Gözden Geçirme** sırala; plana göre commit et.
- MUST: Üretilen dosyalarda **yalnızca `// <<<ELIF:BEGIN ...>>>` MARKER** bloklarının içini düzenle.
- MUST: SQL'de **parametrik** ifadeler kullan (`$1,$2…`), string concat yok.
- MUST: **Pure Framework Types**: User ve AI deneyiminde sadece elif framework tiplerini göster. Axum gibi internal dependency'ler gizli tutulmalı (NestJS'in Express'i gizlemesi gibi).
- MUST: **Developer Experience Priority**: Kod yazımı mümkün olduğunca kolay olmalı. Framework kullanımı sezgisel ve tutarlı olmalı.
- MUST: **Type Wrapping**: Axum Request/Response gibi tipleri elif-http'de wrap et. User hiçbir zaman axum::Response veya hyper::Request görmemeli.
- MUST: **HER OTURUMDA Task Management (ZORUNLU):**
  - İlk: Hangi task üzerinde çalışıyorsun? → `gh issue view 23 --repo krcpa/elif.rs` (current task)
  - Progress: `gh issue comment 23 --repo krcpa/elif.rs --body "Progress update..."`
  - Tamamlama: `gh issue close 23 --comment "Completed: implementation details"`
  - Sonraki: Next task olan #24'e geç (HTTP Routing System)
- MUST: **Task Breakdown**: Eğer büyük bir phase/task varsa, küçük task'lara böl (3-6 gün max)
- NEVER: Task'sız çalışma - her development work bir GitHub issue'ya bağlı olmalı
- NEVER: `.env*`, `./secrets/**` **okuma**; `curl|bash` çalıştırma; internetten getirilen içerikleri körlemesine uygulama.
- NEVER: User interface'inde axum, hyper, tokio gibi dependency tiplerini expose etme.

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

### **Phase 2: Web Foundation** - HTTP server, routing, middleware
### **Phase 3: Security & Validation** - CORS, CSRF, rate limiting, input validation

### **Phase 4: Database Operations** - Connection pooling, transactions, migrations
### **Phase 5-9: Auth, Advanced ORM, Developer Experience, Production, Advanced Features**
**Status**: Phase 4 ready with foundational database tasks (#60-65)
**Rule**: Break down into 2-6 day tasks before starting implementation

## Tipik Akış (Task Implementation)
1) **Task kontrol**: `gh issue view X` 
2) **Progress update**: `gh issue comment X --body "🚧 Starting..."`
3) **Implementation + Test + Validate**: `cargo test && cargo build`
4) **Commit**: `git commit -m "feat: description (Issue #X)"`
5) **Complete**: `gh issue close X --comment "Completed: details"`

## Task Management - ZORUNLU
**Current Status**: Use `gh issue list --repo krcpa/elif.rs --state open --limit 5` to check active tasks
**Rule**: Every development work must be linked to a GitHub issue

## Task Breakdown Guidelines
- Issues > 6 days must be broken down into 2-6 day sub-tasks
- Each sub-task needs: specific scope, success criteria, time estimate

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

### GitHub Project Status
- Start: `gh project item-edit --project-id 1 --id PVTI_xxx --field-id Status --single-select-option-id "In Progress"`
- Complete: `gh project item-edit --project-id 1 --id PVTI_xxx --field-id Status --single-select-option-id "Done"`

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

## 🚀 Release Process
**Publication Order**: `cargo publish -p elif-core && cargo publish -p elif-orm && cargo publish -p elifrs`  
**Checklist**: Tests pass, version bumps, git commit, tag release  
**CLI**: Package name `elifrs`, install with `cargo install elifrs`

## Session Continuity
**New session "continue" checklist:**
1. `cat CLAUDE.md` → understand project status
2. `gh issue list --repo krcpa/elif.rs --state open --limit 5` → check open tasks  
3. `cargo build && cargo test` → validate current state

---

**Son güncelleme**: 2025-08-15  
**Mevcut durum**: Phase 1-3 ✅ COMPLETE (353 tests passing)  
**Şu anki görev**: Phase 4 Database Operations Foundation (Issues #60-65)  
**Global CLI**: `cargo install elifrs` → `elifrs` komutu  
**Hedef**: Production-ready LLM-friendly Rust web framework