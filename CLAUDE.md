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
- ✅ **Release & Publication**: All crates published to crates.io
  - elif-core v0.1.0
  - elif-introspect v0.1.0
  - elif-codegen v0.1.0
  - elif-orm v0.1.0 (Phase 2 placeholder)
  - elifrs v0.1.1 (CLI - global installation available)
- GitHub Projesi: https://github.com/users/krcpa/projects/1/views/1
- Repository: https://github.com/krcpa/elif.rs
- 17 issue oluşturuldu (Phase 1 tamamlandı)
- 6 fazlık geliştirme planı `/plan` dizininde

**🎯 ŞU ANDAKİ GÖREV:**
**Issue #6: Phase 2.1 - Build full ORM with relationships and query builder**
- ✅ Base Model System (Week 1-2) - COMPLETED
- 🚧 Query Builder Enhancement (Week 3-4) - IN PROGRESS  
- 📋 Relationships System (Week 5-6) - NEXT

## Keşif (her oturumda ilk komutlar) - ZORUNLU
- `cat plan/README.md` → geliştirme planına genel bakış.
- `cat plan/phase2/README.md` → mevcut faz detayları.
- `gh issue list --repo krcpa/elif.rs --state open --limit 5` → aktif issue'lar.
- `gh issue view 6 --repo krcpa/elif.rs` → mevcut issue detayı.
- `gh project item-list 1 --owner @me --limit 10` → proje durumu.
- `cargo build` → mevcut kod durumu kontrol.
- `ls crates/` → framework yapısını anla.

## Çalışma Prensipleri (MUST/NEVER) - ZORUNLU
- MUST: **Plan → Uygulama → Test → Gözden Geçirme** sırala; plana göre commit et.
- MUST: Üretilen dosyalarda **yalnızca `// <<<ELIF:BEGIN ...>>>` MARKER** bloklarının içini düzenle.
- MUST: SQL'de **parametrik** ifadeler kullan (`$1,$2…`), string concat yok.
- MUST: **HER OTURUMDA Issue Management (ZORUNLU):**
  - İlk: Hangi issue üzerinde çalışıyorsun? → `gh issue view <N> --repo krcpa/elif.rs`
  - Progress: `gh issue comment <N> --repo krcpa/elif.rs --body "Progress update..."`
  - Tamamlama: `gh issue close <N> --comment "Completed: implementation details"`
- MUST: **GitHub Proje Durumu Yönetimi**: Her issue ile çalışırken proje durumunu güncelle
- NEVER: Issue'sız çalışma - her development work bir issue'ya bağlı olmalı
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

## Geliştirme Süreci (6 Faz)

### **Phase 1: Architecture Foundation** ✅ (COMPLETED - Issue #1-5)
- ✅ Dependency injection container (service-builder based)
- ✅ Service provider system (lifecycle management)
- ✅ Module system for feature organization (dependency resolution)
- ✅ Configuration management (environment variables)
- ✅ Application lifecycle and bootstrapping (graceful shutdown)
**Sonuç**: 33/33 test geçiyor, production-ready temel

### **Phase 2: Database Layer** 🚧 (NEXT - Issue #6-7, #11-12)
- Full ORM with relationships and query builder
- Connection pooling and transaction management
- Model events and observers
- Database seeding and factory system
**Hedef**: Production-ready database layer

### **Phase 3: Security Core** (Issue #8-9, #13-14)
- Authentication system (JWT, session)
- Authorization with roles and permissions
- Input validation and sanitization
- Security middleware (CORS, CSRF, rate limiting)
**Hedef**: Enterprise-grade security

### **Phase 4-6: Developer Experience, Production Features, Advanced Features**
Detaylar `/plan` dizininde

## Tipik Akış (Phase 2 - GÜNCEL ÖRNEK)
1) **Issue kontrol**: `gh issue view 6` → #6: Phase 2.1: Build full ORM with relationships and query builder
2) **Spesifikasyonu oku**: `cat plan/phase2/README.md` → Week 3-4: Query Builder Foundation
3) **Progress update**: `gh issue comment 6 --body "🚧 Starting Query Builder enhancements..."`
4) **Implementation**: MARKER bloklarını düzenle (`crates/elif-orm/src/query.rs`)
5) **Test yaz**: Unit testler ve entegrasyon testleri
6) **Doğrula**: `cargo test && cargo build`
7) **Commit**: `git commit -m "feat: enhance query builder with advanced features (Issue #6)"`
8) **Progress update**: `gh issue comment 6 --body "✅ Query Builder enhancements completed"`
9) **Sonraki milestone**: Week 5-6 Relationships → devam et

## Phase 2 Systematic Approach - ZORUNLU
**📍 Current: Issue #6 (Phase 2.1) - Week 3-4: Query Builder Enhancement**

**✅ COMPLETED (Week 1-2):**
- Base Model trait with CRUD operations
- QueryBuilder with fluent type-safe API
- Comprehensive error handling
- Primary key support and timestamps

**🚧 IN PROGRESS (Week 3-4):**
- Advanced query builder features (subqueries, unions)
- Performance optimization and query caching  
- Enhanced WHERE conditions and complex queries
- Integration with Model trait improvements

**📋 NEXT (Week 5-6):**
- Relationship system (HasOne, HasMany, BelongsTo, BelongsToMany)
- Eager loading and lazy loading mechanisms
- Relationship constraints and validation

**Çalışma Kuralı**: Her hafta sonunda progress commit + issue comment + sonraki hafta plan

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