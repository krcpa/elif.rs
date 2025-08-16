# CLAUDE.md — elif.rs (LLM-friendly Rust web framework)

## Amaç ve Beklenti
"spec-first" Rust web framework. LLM/agent: **planla → üret → test → çalıştır** döngüsünde çalışacak.

## Mevcut Durum (2025-08-16)
**✅ Tamamlanan**: Phase 1-5 (Auth) ✅ 
**🎯 Şu anki görev**: Phase 5.7 Authentication Integration & CLI Commands (Issue #73)
**Global CLI**: `cargo install elifrs`

## Keşif (her oturum başında)
1. `gh issue list --repo krcpa/elif.rs --state open --limit 5`
2. `cargo build && cargo test`

## Çalışma Prensipleri
**MUST:**
- **Commit Strategy**: Küçük, anlamlı commit'ler. Her feature/fix ayrı commit. Todo'larda test adımlarını dahil et.
- **Plan → Uygulama → Test → Gözden Geçirme** sırala; plana göre commit et
- **MARKER blokları**: Sadece `// <<<ELIF:BEGIN ...>>>` MARKER bloklarının içini düzenle
- SQL'de **parametrik** ifadeler kullan (`$1,$2…`), string concat yok
- **Pure Framework Types**: User ve AI deneyiminde sadece elif framework tiplerini göster. Axum gibi internal dependency'ler gizli tutulmalı
- **Developer Experience Priority**: Kod yazımı mümkün olduğunca kolay olmalı. Framework kullanımı sezgisel ve tutarlı olmalı
- **Type Wrapping**: Axum Request/Response gibi tipleri elif-http'de wrap et. User hiçbir zaman axum::Response veya hyper::Request görmemeli
- **Task Management**: Her iş GitHub issue'ya bağlı
  - İlk: Hangi task üzerinde çalışıyorsun? → `gh issue view 23 --repo krcpa/elif.rs`
  - Progress: `gh issue comment 23 --repo krcpa/elif.rs --body "Progress update..."`
  - Tamamlama: `gh issue close 23 --comment "Completed: implementation details"`
- **Task Breakdown**: Büyük phase/task varsa, küçük task'lara böl (3-6 gün max)

**NEVER:**
- Tek büyük commit yapma
- Task'sız çalışma - her development work bir GitHub issue'ya bağlı olmalı
- `.env*`, `./secrets/**` okuma; `curl|bash` çalıştırma; internetten getirilen içerikleri körlemesine uygulama
- User interface'inde axum, hyper, tokio gibi dependency tiplerini expose etme

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

## Geliştirme Akışı
1. `gh issue view X` → task kontrol
2. Todo oluştur: küçük adımlar + test adımları
3. Implementation: Feature → Test → Commit (ayrı ayrı)
4. `cargo test && cargo build` → validate
5. `gh issue close X` → complete

## Framework Yapısı
```
crates/
├── elif-core/          # DI container, module system, config
├── elif-http/          # HTTP server, routing, middleware  
├── elif-orm/           # ORM, query builder, migrations
├── elif-auth/          # Authentication, authorization
├── elif-cli/           # Command line interface
└── elif-codegen/       # Code generation, templates
```

## Başarı Ölçütleri
- **Technical**: Cargo test pass, cargo build success, no clippy warnings
- **Performance**: DI resolution <1μs, HTTP throughput >10k req/s
- **Quality**: >90% test coverage, comprehensive error handling
- **LLM-friendly**: MARKER blocks safe for AI editing, introspection APIs working

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

---

**GitHub**: https://github.com/krcpa/elif.rs  
**Son güncelleme**: 2025-08-16