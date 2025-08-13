# elif.rs — LLM-Friendly Rust Web Framework

**Product Requirements Document (PRD) · v0.1**
**Owner:** Anıl · **Audience:** Core devs + AI agent (Claude)
**Hedef:** LLM/agent’ların *geliştirme sürecinde* zorlanmadan kaliteli app çıkarabilmesi (spec-first, introspektif, deterministik iskelet, düşük sihir).

---

## 0) Özet (1 paragraf)

elif.rs; Rust’ta Axum + sqlx temelli, **spec-first** bir web framework’tür. Kaynak (Resource) tanımı tek bir YAML dosyasından yapılır; framework bu spec’ten **model, migration, handler iskeleti (MARKER’lı), test, OpenAPI ve introspeksiyon artefaktlarını** üretir. **CLI** yüzeyi kısa ve deterministiktir. Amaç: “Todo app yaz” gibi görevlerde bir LLM/agent’ın **3 dosyadan fazla** dokunmadan, ilk çalıştırmada **cargo hata sayısı ≤1** olacak şekilde uygulamayı ayağa kaldırmasıdır.

---

## 1) Amaçlar / Başarı Kriterleri

* **Spec-first DX:** `resources/*.resource.yaml` → codegen ile **idempotent** üretim.
* **LLM-friendly düzenleme:** Kaynak dosyada ve kodda **MARKER** blokları; agent yalnızca bu alanlara yazar.
* **Introspeksiyon:** `/_map.json`, `/_openapi.json`, `/_schemas/*`, `/_db/schema.sql` uçları veya dosyaları mevcut.
* **Hata modeli:** Stabil `code + hint`; `.elif/errors.yaml` tek kaynaktır.
* **CLI sözleşmesi:** `elif generate`, `elif check`, `elif test --focus …`, `elif map`, `elif openapi export`.
* **MVP kabul:**

  * `Todo` örneği tek spec + tek komutla (generate) oluşur, testler koşar.
  * İlk `cargo check` çıktısı, `elif fix` önerileriyle otomatik düzeltilebilir olur.
  * OpenAPI `/ _openapi.json` ve Swagger UI `/ _ui` çalışır.

**Başarı ölçütleri**

* Tek prompt → Todo: **build success** ve **temel HTTP happy path testi geçer**.
* Agent’ın düzenlediği dosya sayısı: **≤ 3** (spec, 1-2 MARKER bloğu, opsiyonel test).
* `elif check` < 5 sn (soğuk cache hariç); `generate` idempotent diffs.

**Non-Goals (MVP)**

* Auth/RBAC, rate limit’in gerçek uygulanması (yalnızca deklaratif metadata).
* Background jobs/queue.
* Full MCP implementasyonu (M1+).
* Production hardening (TLS termination, multi-tenancy vs.).

---

## 2) Kullanıcılar / Personalar

* **AI Agent (Claude):** Kod üretir, dosya düzenler, CLI komutları çalıştırır; **deterministik** arayüz ister.
* **Senior Backend Dev:** Rust/Axum/sqlx bilir; codegen’in dışına gerektiğinde **escape hatch** ister.
* **Tech Writer/QA:** OpenAPI ve `_map.json` üzerinden coverage ve dökümantasyon doğrular.

---

## 3) Kapsam ve Sürümleme

* **MVP (v0.1)**: Spec → codegen (model, handler skeleton, migration, test), CLI, introspeksiyon, swagger.
* **M1 (v0.2)**: `elif fix` (compiler hata eyleyicisi), schema drift kontrolü, `list` için cursor paging + filter/search/order.
* **M2 (v0.3)**: MCP tool’ları (scaffold.resource, generate, run.tests, fix), policy middleware iskeleti, sqlite test modu.
* **M3 (v0.4)**: Observability genişletme (OTel), minimal job runner, error catalog tooling.

---

## 4) Mimarî (yüksek seviye)

```
elif.rs/
├─ Cargo.toml                # [workspace]
├─ .elif/
│  ├─ manifest.yaml          # proje manifesti
│  ├─ errors.yaml            # stabil hata kodları
│  └─ policies.yaml          # deklaratif policy
├─ resources/                # tek hakikat kaynağı (ResourceSpec)
│  └─ *.resource.yaml
├─ apps/
│  └─ api/                   # Axum HTTP app
├─ crates/
│  ├─ core/                  # error, config, tracing helpers
│  ├─ orm/                   # sqlx wrapper + models
│  ├─ codegen/               # spec → code/migration/test/openapi
│  ├─ introspect/            # _map.json, _schemas, _db/schema.sql
│  └─ cli/                   # elif komutu
└─ migrations/               # sqlx migration’ları
```

* **HTTP**: Axum (Hyper)
* **DB**: sqlx (offline)
* **Şema/Doküman**: schemars + utoipa → OpenAPI
* **Şablonlama**: tinytemplate (hafif, deterministik)
* **Günlükleme**: tracing (JSON line)

**Karar Matrisi (özet)**

| Bileşen    | Seçenek          | Uygunluk | Not                                          |
| ---------- | ---------------- | -------: | -------------------------------------------- |
| HTTP       | **Axum**         |  **90%** | Minimal sihir, güçlü ekosistem               |
| ORM        | **sqlx**         |  **85%** | Compile-time check, ham SQL görünür          |
| Doc        | **utoipa**       |  **80%** | Hızlı OpenAPI üretimi                        |
| Templating | **tinytemplate** |  **85%** | Basit, hızlı; gerekirse v2: askama/minijinja |

---

## 5) ResourceSpec (tek kaynak dosyası)

### 5.1 Örnek

```yaml
# resources/todo.resource.yaml
kind: Resource
name: Todo
route: /todos
storage:
  table: todos
  soft_delete: false
  timestamps: true
  fields:
    - { name: id,    type: uuid, pk: true, default: gen_random_uuid() }
    - { name: title, type: text, required: true, validate: { min: 1, max: 120 } }
    - { name: done,  type: bool, default: false, index: true }
indexes:
  - { name: idx_todos_done, fields: [done] }
uniques: []
relations: []
api:
  operations:
    - { op: create, method: POST,   path: "/" }
    - { op: list,   method: GET,    path: "/", paging: cursor, filter: [done], search_by: [title], order_by: [created_at] }
    - { op: get,    method: GET,    path: "/:id" }
    - { op: update, method: PATCH,  path: "/:id" }
    - { op: delete, method: DELETE, path: "/:id" }
policy:
  auth: public
  rate_limit: "100/m"
validate:
  constraints:
    - { rule: "title != ''", code: EMPTY_TITLE, hint: "Provide non-empty title" }
examples:
  create: { title: "Buy milk" }
events:
  emit: [created, updated, deleted]
```

### 5.2 Tip eşlemleri (Rust/SQL)

* `uuid ↔ uuid ↔ Uuid`
* `text/string(max) ↔ text/varchar ↔ String`
* `bool ↔ boolean ↔ bool`
* `int/bigint ↔ integer/bigint ↔ i32/i64`
* `float ↔ double precision ↔ f64`
* `numeric(p,s) ↔ numeric(p,s) ↔ Decimal (opsiyon)`
* `timestamp/timestamptz ↔ time crate tipleri`
* `json ↔ jsonb ↔ serde_json::Value`
* `array<T> ↔ T[]`

### 5.3 Operasyon sözleşmesi

* `list.paging = cursor` (opaque token; `items` + `next`)
* `filter/search_by/order_by` **beyaz liste** alan adlarıdır.
* `policy.auth ∈ {public,user,service,role:<x>}` (MVP: metadata; M2: middleware)
* `validate.constraints[rules]` basit ifadeler (MVP: doğrulama ipucu; handler içine kopyalanır).

### 5.4 JSON Schema (ResourceSpec)

MVP’de minimal bir JSON Schema sağlanır; `elif check` bununla doğrular. (Ayrıntılı şema ek dosyada; Claude implementasyonunda kullanılacak.)

---

## 6) Codegen (detay)

### 6.1 Üretilen artefaktlar

* **Model:** `crates/orm/src/models/<resource>.rs` (serde + schemars)
* **Handler skeleton:** `apps/api/src/routes/<resource>.rs` (**MARKER** bloklarıyla)
* **Migration:** `migrations/<epoch>_create_<table>.sql`
* **Test:** `tests/<resource>_http.rs` (happy/unhappy path iskeleti)
* **OpenAPI parçası:** utoipa derive’ları; birleşim → `/_openapi.json`

### 6.2 MARKER sözleşmesi

```text
// <<<ELIF:BEGIN agent-editable:<id>>>
// (Agent yalnızca bu bloğu düzenler; codegen korur.)
// <<<ELIF:END agent-editable:<id>>>
```

* Codegen, mevcut dosyada aynı `<id>` içeriğini **yerinde** tutar, dışını günceller (idempotent merge).

### 6.3 İdempotent yazım

* `write_if_changed` (no-op diff)
* `write_preserving_markers` (regex ile blok koruma)
* `routes/mod.rs` için `append_once`

### 6.4 Migration üretimi

* Tablo, indeks, unique, opsiyonel `created_at/updated_at`, `deleted_at`.
* **Naming:** `create_<table>`, `idx_*`, `uq_*`.
* **Postgres hedefi** (MVP); M2’de sqlite seçeneği.

---

## 7) HTTP Uygulaması

### 7.1 Router

* Her resource kendi `router()` fonksiyonu ile mount edilir.
* Global: `/ _openapi.json`, `/ _ui` (Swagger UI), `/ _map.json`, `/ _schemas/*`, `/ _db/schema.sql`.

### 7.2 Operation IDs & Dokümantasyon

* Utoipa ile `paths(...)`, `components(...)`;
* OpenAPI çıktısına `x-elif.location` (dosya/satır), `x-elif.hint` (agent ipucu) vendor extension’ları eklenir.

---

## 8) Introspeksiyon Artefaktları

### 8.1 `_map.json` (şema)

```json
{
  "routes": [
    {
      "op_id": "Todo.create",
      "method": "POST",
      "path": "/todos",
      "file": "apps/api/src/routes/todo.rs",
      "marker": "create_Todo"
    }
  ],
  "models": [{"name":"Todo","file":"crates/orm/src/models/todo.rs"}],
  "specs":  [{"name":"Todo","file":"resources/todo.resource.yaml"}]
}
```

### 8.2 `_schemas/*`

* JSON Schema dosyaları (model tipleri).

### 8.3 `_db/schema.sql`

* En güncel DB şeması (sqlx metadata’dan veya migration birleşiminden üretilir).

---

## 9) Hata Modeli

### 9.1 `.elif/errors.yaml`

```yaml
- code: EMPTY_TITLE
  http: 400
  message: "title cannot be empty"
  hint: "Provide non-empty title"
```

### 9.2 HTTP hata gövdesi (MVP)

```json
{
  "error": {
    "code": "EMPTY_TITLE",
    "message": "title cannot be empty",
    "hint": "Provide non-empty title"
  }
}
```

* Stabil `code`; `hint` ajan için aksiyon cümlesi.
* OpenAPI’de `x-elif.error` ile referans.

---

## 10) DB & ORM

* **sqlx (offline)**: `SQLX_OFFLINE=true` ile deterministik derleme.
* **Query DSL (min):** MVP’de ham parametrik SQL; M1’de `list` için basit builder (filter/search/order/paging).
* **Explain yardımcıları (M1+):** `orm::explain(query)` → `EXPLAIN` çıktısı.
* **Drift kontrolü (M1):** Spec’ten `schema.sql` üret; canlı şema ile diff → `elif check` kırar.

---

## 11) CLI Spesifikasyonu

| Komut                                                       | Açıklama                                                                 | Örnek                                                                 |
| ----------------------------------------------------------- | ------------------------------------------------------------------------ | --------------------------------------------------------------------- |
| `elif generate`                                             | `resources/*.yaml` → code + migration + test + openapi                   | `elif generate`                                                       |
| `elif resource new <Name> --route /x --fields a:int,b:text` | Yeni spec taslağı yazar                                                  | `elif resource new Todo --route /todos --fields title:text,done:bool` |
| `elif check`                                                | cargo fmt+clippy, spec schema validate, sqlx offline verify, drift check | `elif check`                                                          |
| `elif test [--focus <res>]`                                 | Tüm veya ilgili testleri koşar                                           | `elif test --focus todo`                                              |
| `elif map --json`                                           | `_map.json` üretir (stdout veya `target/`)                               | `elif map --json`                                                     |
| `elif openapi export`                                       | OpenAPI birleştir, dosyaya yaz                                           | `elif openapi export`                                                 |
| `elif fix --from-cargo <path>` (M1)                         | compiler hatalarını eyleme çeviren JSON aksiyonları                      | `elif fix --from-cargo target/check.json`                             |

**Çıkış kodları**

* `0`: başarı / öneri yok
* `1`: doğrulama/derleme/migration hatası
* `2`: drift tespit edildi (aksiyon önerisi mevcut)

**`elif fix` JSON çıktısı (örnek)**

```json
{"actions":[{"file":"apps/api/src/routes/todo.rs","marker":"create_Todo","suggest":"Add `use crate::models::CreateTodo;`"}]}
```

---

## 12) Test Stratejisi

* **Unit:** Model/utility fonksiyonları.
* **Integration (HTTP):** Happy/Unhappy path; `tests/<res>_http.rs`.
* **Contract:** OpenAPI’ya uyum (status/body/layout).
* **Golden tests:** Örnek istek/yanıt dosyaları.
* **DB:** Postgres (MVP); M2’de sqlite test modu (hız).

---

## 13) Observability

* **tracing (JSON lines)**: `operation_id`, `request_id`, `duration_ms`, `status`, `error.code`.
* `/ _health` (opsiyonel): basit çalışırlık.

---

## 14) Güvenlik (MVP sınırları)

* Secrets yalnızca env/secret manager (kodda yok).
* `policy` metadata; enforcement M2’de middleware ile.
* Codegen, raw string SQL’de **parametrizasyon** zorunlu (LLM injection’ı önlemek için).

---

## 15) Performans & Kısıtlar

* Geliştirici deneyimi odaklı; **soğuk build** süreleri normal Rust seviyesinde kabul.
* RPS hedefleri MVP kapsam dışı; Axum default yeterli.

---

## 16) Kabul Kriterleri (MVP)

1. `resources/todo.resource.yaml` eklendiğinde `elif generate`

   * model/handler/migration/test üretsin (idempotent).
2. `cargo run -p elif-api` → server `0.0.0.0:8080`’de açılır.
3. `POST /todos` başarılı; boş title’da hata gövdesi **stabil code+hint** ile döner.
4. `GET /_openapi.json` ve `/_ui` çalışır.
5. `GET /_map.json` route↔dosya↔marker eşleşmesini verir.
6. `elif check` başarı; drift yokken `0`, drift varken `2` döner.

---

## 17) Riskler & Mitigasyon

| Risk                  | Etki              | Çözüm                                        |
| --------------------- | ----------------- | -------------------------------------------- |
| Spec karmaşıklaşır    | DX düşer          | v0.1’de minimal alanlar; v2’de extension’lar |
| Marker koruma hatası  | Kod kaybı         | Regex-tabanlı merge + snapshot test          |
| sqlx metadata drift   | Build kırılır     | `elif check` drift dedektörü + öneri         |
| Utoipa sürüm değişimi | Doküman kırılması | PR pin + bütünleşik kontrat testleri         |

---

## 18) Yol Haritası / İş Paketleri

**Sprint 1 (MVP Core)**

* [ ] `crates/codegen`: spec parse, model/handler/migration/test üretimi
* [ ] MARKER koruma & idempotent yazım
* [ ] `apps/api` temel server + `/ _openapi.json` + `/ _ui`
* [ ] `crates/introspect`: `_map.json` üretimi
* [ ] `crates/cli`: `generate`, `resource new`, `map`, `openapi export`
* [ ] `Todo` örneği + temel HTTP test

**Sprint 2 (DX Güçlendirme)**

* [ ] `elif check` (fmt, clippy, spec schema validate, sqlx offline verify)
* [ ] `list` için cursor paging + filter/search/order builder
* [ ] `.elif/errors.yaml` entegrasyonu ve runtime hata modeli
* [ ] `_db/schema.sql` üretimi & drift kontrolü

**Sprint 3 (Agent Entegrasyonu & Policy)**

* [ ] `elif fix` (compiler error mapper)
* [ ] Policy attribute → middleware iskeleti (noop)
* [ ] MCP tool taslağı (komutların uzaktan çağrımı; M2)

---

## 19) Açık Sorular

* `validate.constraints.rule` için ifade dili: Basit string mi, küçük bir DSL mi? (MVP: string + örnek kod üretim)
* `events.emit` nereye publish edilecek? (M3: outbox/table + job runner)
* OpenAPI vendor extension alanları (`x-elif.*`) için minimum set?

---

## 20) Ekler

### 20.1 `_map.json` JSON şeması (özet)

```json
{
  "type":"object",
  "properties":{
    "routes":{"type":"array","items":{
      "type":"object",
      "required":["op_id","method","path","file"],
      "properties":{
        "op_id":{"type":"string"},
        "method":{"type":"string"},
        "path":{"type":"string"},
        "file":{"type":"string"},
        "marker":{"type":"string"}
      }
    }},
    "models":{"type":"array"},
    "specs":{"type":"array"}
  }
}
```

### 20.2 Hata Gövdesi JSON şeması

```json
{
  "type":"object",
  "required":["error"],
  "properties":{
    "error":{
      "type":"object",
      "required":["code","message"],
      "properties":{
        "code":{"type":"string"},
        "message":{"type":"string"},
        "hint":{"type":"string"}
      }
    }
  }
}
```

### 20.3 CLI Çıktı Sözleşmeleri

* `elif map --json` → `_map.json` **stdout**
* `elif openapi export` → `target/_openapi.json` (path konfigürasyonu `.elif/manifest.yaml`)

---

## 21) Sonraki Adımlar (uygulanabilir)

* Claude’a: **Sprint 1** iş paketlerini sırayla uygula.
* İlk PR: workspace + crates iskeletleri + CLI kabuğu + codegen’in model/handler/migration üretimi.
* İkinci PR: introspect `_map.json` + `/ _openapi.json` + `/ _ui`.
* Üçüncü PR: Todo örneği + testler + `elif generate` uçtan uca.

> MCP entegrasyonunu M2’de ele alacağız; bu PRD, **geliştirme sürecini LLM-friendly** kılan çekirdeği tanımlar.

