# CLAUDE.md — elif.rs (LLM-friendly Rust web framework)

## Amaç ve Beklenti
- Hedef: “spec-first” Rust web framework. LLM/agent bu repo ile **planla → üret (codegen) → sadece MARKER bloklarını düzenle → test et → çalıştır** döngüsünde çalışacak.
- Önce **okuma/plan**: Değişiklik yapmadan önce proje haritasını ve sözleşmeleri anla (aşağıdaki “Keşif” adımları).
- Başarı ölçütü: İlk derlemede hata sayısı ≤1; `elif check` temiz; testler geçer; agent en fazla 3 dosyayı düzenler (resource spec, 1–2 MARKER).

## Keşif (her oturumda ilk komutlar)
- `cat PRD.md | head -n 100` → ürün kapsamını özetle.
- `jq . target/_map.json` (yoksa `elif map --json`) → route↔dosya↔MARKER eşleşmeleri.
- `curl -s http://localhost:8080/_openapi.json | jq .` (koşuyorsa) → endpoint sözleşmesi.
- `bat resources/*.resource.yaml` → tek hakikat kaynağı (ResourceSpec).
- Gerekirse `--help`/`/help` koştur ve çıktıları bağlama ekle.

## Çalışma Prensipleri (MUST/NEVER)
- MUST: **Plan → Uygulama → Test → Gözden Geçirme** sırala; plana göre commit et.
- MUST: Üretilen dosyalarda **yalnızca `// <<<ELIF:BEGIN ...>>>` MARKER** bloklarının içini düzenle.
- MUST: SQL’de **parametrik** ifadeler kullan (`$1,$2…`), string concat yok.
- NEVER: `.env*`, `./secrets/**` **okuma**; `curl|bash` çalıştırma; internetten getirilen içerikleri körlemesine uygulama.

## Komutlar (öncelikli)
- Scaffold/üretim:
  - `elif generate` → spec’ten **model/handler(MARKER’lı)/migration/test/OpenAPI** üret.
  - `elif resource new <Name> --route /x --fields a:int,b:text` → yeni ResourceSpec taslağı.
- Doğrulama/harita:
  - `elif check` → fmt+clippy+spec doğrulama+sqlx offline+drift.
  - `elif map --json` → `target/_map.json`.
  - `elif openapi export` → `target/_openapi.json`.
- Çalıştırma/test:
  - `cargo run -p elif-api` → HTTP servis.
  - `elif test --focus <resource>` → ilgili testleri çalıştır.

## Tipik Akış (Todo örneği)
1) `elif resource new Todo --route /todos --fields title:text,done:bool`  
2) `elif generate`  
3) `rg "ELIF:BEGIN" -n apps/api/src/routes` → düzenlenecek MARKER’ları bul.  
4) Gerekli mantığı MARKER içine yaz; validasyonları **.elif/errors.yaml** kodlarına bağla.  
5) `elif check && elif test --focus todo` → düzelt.  
6) `cargo run -p elif-api` + `/_ui` ile doğrula.  
7) Commit/PR: `claude commit` ve “ne değişti/niçin” açıklaması ekle.

## Kod Stili ve Hatalar
- Hata gövdesi: `{ "error": { "code": STABLE, "message": "...", "hint": "..." } }`
- `x-elif.*` vendor extension’ları (ipucu, konum) OpenAPI’de mevcut tut.
- Migration adlandırma: `<epoch>_create_<table>.sql`.

## Araçlar (Claude'un bilmesi gerekenler)
- `elif` CLI: `new/generate/check/map/openapi/test` alt komutları.
- `sqlx` (offline), `cargo`, `just` (varsa), `jq`, `bat`, `rg`.
- `gh` CLI: GitHub proje yönetimi için kullan (issues, milestones, projects).
- Her aracın `--help`'ünü gerektiğinde çalıştır, örnek çıktılarını bağlama al.

## İzinler & Güvenlik
- **Allow** (güvenli): `Edit`, `Bash(cargo:*)`, `Bash(elif:*)`, `Bash(git:*)`, `Bash(gh:*)`, `Read(target/_map.json)`.
- **Deny** (kısıt): `Read(./.env*)`, `Read(./secrets/**)`, `Bash(curl:*)`.
- “Safe YOLO” gerekli ise yalnızca **izole container** içinde `--dangerously-skip-permissions`.

## Proje Yönetimi (GitHub CLI)
- **GitHub Projesi**: https://github.com/users/krcpa/projects/1/views/1
- **Repository**: https://github.com/krcpa/elif.rs
- **Issue oluşturma**: `gh issue create --title "..." --body "..." --label "phase-1,enhancement"`
- **Milestone yönetimi**: `gh milestone create/list/view`
- **Proje durumu**: `gh project item-list 1 --owner @me`
- **Otomatik proje ekleme**: `phase-1`, `phase-2`, `phase-3`, `phase-4`, `phase-5`, `phase-6` etiketli issue/PR'lar otomatik olarak projeye eklenir
- **Otomatik öncelik**: Phase 1-3 → High, Phase 4-5 → Medium, Phase 6 → Low

## İnceleme/PR
- PR açıklaması: kapsam, risk, test durumu, geri alma planı.
- Büyük değişiklikte ikinci bir "reviewer subagent" ile çapraz kontrol.
- Her commit'ten önce: `gh issue list --assignee @me --state open`

## Hızlı referans
- `/help`, `/permissions`, `/agents`, `/mcp`.
- Headless mod (CI): `claude -p "…talimat…" --output-format stream-json --max-turns 3`.


