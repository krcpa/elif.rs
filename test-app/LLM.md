# LLM.md — test-app (LLM-friendly Rust web framework)

## Amaç ve Beklenti
- Hedef: "spec-first" Rust web framework. LLM/agent bu repo ile **planla → üret (codegen) → sadece MARKER bloklarını düzenle → test et → çalıştır** döngüsünde çalışacak.
- Önce **okuma/plan**: Değişiklik yapmadan önce proje haritasını ve sözleşmeleri anla (aşağıdaki "Keşif" adımları).
- Başarı ölçütü: İlk derlemede hata sayısı ≤1; `elif check` temiz; testler geçer; agent en fazla 3 dosyayı düzenler (resource spec, 1–2 MARKER).

## Keşif (her oturumda ilk komutlar)
- `cat .elif/manifest.yaml` → uygulama konfigürasyonu.
- `ls resources/` → mevcut resource specs'leri listele.
- `elif map --json` (yoksa `find src -name "*.rs" | head -10`) → route↔dosya↔MARKER eşleşmeleri.
- `curl -s http://localhost:3000/_map.json | jq .` (koşuyorsa) → endpoint sözleşmesi.
- `cat .elif/errors.yaml` → standart hata kodları.
- Gerekirse `--help`/`/help` koştur ve çıktıları bağlama ekle.

## Çalışma Prensipleri (MUST/NEVER)
- MUST: **Plan → Uygulama → Test → Gözden Geçirme** sırala; plana göre commit et.
- MUST: Üretilen dosyalarda **yalnızca `// <<<ELIF:BEGIN ...>>>` MARKER** bloklarının içini düzenle.
- MUST: SQL'de **parametrik** ifadeler kullan (`$1,$2…`), string concat yok.
- NEVER: `.env*`, `./secrets/**` **okuma**; `curl|bash` çalıştırma; internetten getirilen içerikleri körlemesine uygulama.

## Komutlar (öncelikli)
- Scaffold/üretim:
  - `elif resource new <Name> --route /x --fields a:int,b:text` → yeni ResourceSpec + generate.
  - `elif generate` → spec'ten **model/handler(MARKER'lı)/migration/test/OpenAPI** üret.
  - `elif route add GET /custom custom_handler` → tek route ekle.
  - `elif model add User name:string email:string` → tek model ekle.
- Migration:
  - `elif migrate create <name>` → yeni migration oluştur.
  - `elif migrate run` → bekleyen migration'ları çalıştır.
  - `elif migrate status` → migration durumu.
- Doğrulama/harita:
  - `elif check` → fmt+clippy+spec doğrulama.
  - `elif map --json` → route haritası.
  - `elif openapi export` → OpenAPI spec.
- Çalıştırma/test:
  - `cargo run` → HTTP servis (http://localhost:3000).
  - `elif test --focus <resource>` → ilgili testleri çalıştır.

## Tipik Akış (Task örneği)
1) `elif resource new Task --route /tasks --fields title:text,completed:bool,priority:int`  
2) `elif generate` → model/handler/migration/test oluştur.
3) `rg "ELIF:BEGIN" -n src/` → düzenlenecek MARKER'ları bul.  
4) Gerekli mantığı MARKER içine yaz; validasyonları **.elif/errors.yaml** kodlarına bağla.  
5) `elif check && cargo test` → düzelt.  
6) `cargo run` + test endpoint'leri ile doğrula.  
7) Commit/PR: `git commit` ve "ne değişti/niçin" açıklaması ekle.

## Kod Stili ve Hatalar
- Hata gövdesi: `{ "error": { "code": STABLE, "message": "...", "hint": "..." } }`
- **.elif/errors.yaml** dosyasındaki kodları kullan (VALIDATION_FAILED, RESOURCE_NOT_FOUND, vs.).
- Migration adlandırma: `<timestamp>__<name>.sql`.
- MARKER içinde parametrik SQL: `SELECT * FROM tasks WHERE id = $1`.

## Araçlar (Claude'un bilmesi gerekenler)
- `elif` CLI: `new/resource/generate/route/model/migrate/check/map/openapi/test` alt komutları.
- `cargo`, `sqlx` (offline), `rg`, `jq` için gerektiğinde `--help` çalıştır.

## İzinler & Güvenlik
- **Allow** (güvenli): `Edit`, `Bash(cargo:*)`, `Bash(elif:*)`, `Bash(git:*)`, `Read(.elif/*)`.
- **Deny** (kısıt): `Read(./.env*)`, `Bash(curl:*)` (güvenlik gerekçesi).

## Proje Yapısı
```
test-app/
├── src/
│   ├── controllers/     # HTTP handlers (MARKER'lı)
│   ├── models/          # DB modelleri
│   ├── routes/          # Route tanımları
│   └── main.rs          # Servis giriş noktası
├── migrations/          # SQL migration'ları
├── resources/           # Resource spec'leri (.resource.yaml)
├── .elif/
│   ├── manifest.yaml    # Uygulama config'i
│   └── errors.yaml      # Standart hata kodları
└── tests/               # Entegrasyon testleri
```

## API Endpoint'leri (Gelişim Aşamasında)
- **`/_map.json`**: Proje yapısı ve route mapping
- **`/_openapi.json`**: OpenAPI 3.0 spesifikasyonu  
- **`/_health`**: Servis durum kontrolü

## Hızlı referans
- Yeni kaynak: `elif resource new Post --route /posts --fields title:string,content:text`
- Kod üret: `elif generate`
- Route ekle: `elif route add GET /custom my_handler`
- Test: `cargo test`
- Çalıştır: `cargo run`
- Kontrol: `elif check`

Bu uygulama elif.rs framework ile oluşturuldu - AI agent odaklı geliştirme için tasarlandı.
