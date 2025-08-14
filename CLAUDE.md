# CLAUDE.md — elif.rs (LLM-friendly Rust web framework)

## Amaç ve Beklenti
- Hedef: "spec-first" Rust web framework. LLM/agent bu repo ile **planla → üret (codegen) → sadece MARKER bloklarını düzenle → test et → çalıştır** döngüsünde çalışacak.
- Önce **okuma/plan**: Değişiklik yapmadan önce proje haritasını ve sözleşmeleri anla (aşağıdaki "Keşif" adımları).
- Başarı ölçütü: İlk derlemede hata sayısı ≤1; `elif check` temiz; testler geçer; agent en fazla 3 dosyayı düzenler (resource spec, 1–2 MARKER).

## Proje Durumu (GÜNCEL - 2025-01-13)
**✅ HAZIR BILEŞENLER:**
- Framework temeli: CLI yapısı, temel scaffold, MARKER sistemi
- GitHub Projesi: https://github.com/users/krcpa/projects/1/views/1
- Repository: https://github.com/krcpa/elif.rs
- 17 issue oluşturuldu ve projeye eklendi
- 6 fazlık geliştirme planı `/plan` dizininde
- GitHub Actions otomatik proje yönetimi çalışıyor

**🎯 ŞU ANDAKİ GÖREV:**
Phase 1 geliştirmeye başla - Issue #1: Design dependency injection system
Detaylar: `/plan/phase1/SPECIFICATIONS.md` içinde

## Keşif (her oturumda ilk komutlar)
- `cat plan/README.md` → geliştirme planına genel bakış.
- `cat plan/phase1/README.md` → mevcut faz detayları.
- `gh issue list --repo krcpa/elif.rs --state open --limit 5` → aktif issue'lar.
- `gh project item-list 1 --owner @me --limit 10` → proje durumu.
- `cargo build` → mevcut kod durumu kontrol.
- `ls crates/` → framework yapısını anla.

## Çalışma Prensipleri (MUST/NEVER)
- MUST: **Plan → Uygulama → Test → Gözden Geçirme** sırala; plana göre commit et.
- MUST: Üretilen dosyalarda **yalnızca `// <<<ELIF:BEGIN ...>>>` MARKER** bloklarının içini düzenle.
- MUST: SQL'de **parametrik** ifadeler kullan (`$1,$2…`), string concat yok.
- MUST: GitHub issue'ları güncel tut - tamamladığında `gh issue close #N --comment "Completed: ..."`
- NEVER: `.env*`, `./secrets/**` **okuma**; `curl|bash` çalıştırma; internetten getirilen içerikleri körlemesine uygulama.

## Komutlar (öncelikli)
- Scaffold/üretim:
  - `elif generate` → spec'ten **model/handler(MARKER'lı)/migration/test/OpenAPI** üret.
  - `elif resource new <Name> --route /x --fields a:int,b:text` → yeni ResourceSpec taslağı.
  - `elif new <app-name>` → yeni uygulama oluştur.
- Migration:
  - `elif migrate create <name>` → yeni migration oluştur.
  - `elif migrate run` → bekleyen migration'ları çalıştır.
  - `elif migrate status` → migration durumu.
- Doğrulama/harita:
  - `elif check` → fmt+clippy+spec doğrulama.
  - `elif map --json` → route haritası.
  - `elif openapi export` → OpenAPI spec.
- Çalıştırma/test:
  - `cargo run` → HTTP servis (localhost:3000).
  - `elif test --focus <resource>` → ilgili testleri çalıştır.

## Geliştirme Süreci (6 Faz)

### **Phase 1: Architecture Foundation** (CURRENT - Issue #1-5)
- Dependency injection container
- Service provider system
- Module system for feature organization
- Configuration management
- Application lifecycle and bootstrapping
**Hedef**: Sağlam mimari temel

### **Phase 2: Database Layer** (Issue #6-7, #11-12)
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

## Tipik Akış (Phase 1 örneği)
1) **Issue seç**: `gh issue view 1` → #1: Design dependency injection system
2) **Spesifikasyonu oku**: `cat plan/phase1/SPECIFICATIONS.md`
3) **Implementation**: `crates/elif-core/src/container.rs` oluştur
4) **Test yaz**: Unit testler ve entegrasyon testleri
5) **Doğrula**: `cargo test && cargo build`
6) **Commit**: `git commit -m "feat: implement dependency injection container"`
7) **Issue kapat**: `gh issue close 1 --comment "Completed DI container with full test coverage"`
8) **Sonraki issue**: `gh issue view 2` → devam et

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

## Hızlı referans
- `/help`, `/permissions`, `/agents`, `/mcp`.
- Headless mod (CI): `claude -p "…talimat…" --output-format stream-json --max-turns 3`.

---

**Son güncelleme**: 2025-01-13  
**Mevcut faz**: Phase 1 - Architecture Foundation  
**İlk görev**: Issue #1 - Design dependency injection system  
**Hedef**: Production-ready LLM-friendly Rust web framework