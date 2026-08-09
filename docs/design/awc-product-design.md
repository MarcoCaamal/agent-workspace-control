# Agent Workspace Control (AWC)

## Documento de diseño técnico

**Nombre del producto:** Agent Workspace Control
**CLI:** `awctl`
**Lenguaje principal:** Rust
**Tipo:** herramienta local-first para gobernanza de workspaces de agentes de IA
**Estado:** diseño propuesto
**Plataforma primaria:** Linux
**Plataforma secundaria:** macOS
**Windows:** experimental inicialmente
**Fecha:** 8 de agosto de 2026

---

# 1. Resumen ejecutivo

Agent Workspace Control, abreviado **AWC**, es un sistema local diseñado para gobernar el filesystem de agentes de IA que trabajan durante periodos prolongados dentro de un workspace persistente.

Su propósito es solucionar un problema que aparece cuando un agente deja de utilizarse únicamente como chatbot y comienza a trabajar como asistente persistente:

* crea documentos;
* genera planes;
* realiza investigaciones;
* produce code reviews;
* mantiene trabajo pendiente;
* crea archivos temporales;
* reutiliza información entre sesiones;
* interactúa con varios proyectos;
* necesita localizar credenciales y conexiones;
* modifica continuamente su workspace.

Sin una capa de gobernanza, ese workspace tiende a degradarse:

```text
workspace/
├── plan.md
├── nuevo-plan.md
├── plan-final.md
├── review.md
├── notes.md
├── investigation.md
├── temp.md
├── result.json
├── feature-backend-frontend-final.md
└── ...
```

El problema fundamental es que una instrucción como:

> Mantén tu workspace limpio.

es únicamente una **convención de comportamiento**.

No existe una garantía determinista de que el agente vaya a cumplirla.

AWC introduce una capa independiente entre el agente y los artefactos persistentes del workspace:

```text
                User
                  │
                  ▼
              AI Agent
                  │
                  ▼
        Agent Workspace Control
                  │
       ┌──────────┼──────────┐
       ▼          ▼          ▼
    Metadata   Policies   Lifecycle
       │          │          │
       └──────────┼──────────┘
                  ▼
              Workspace
```

La responsabilidad se divide de forma explícita:

> **El agente decide qué quiere producir. AWC gobierna dónde vive, cómo se registra, cómo se relaciona y qué sucede con ello durante su ciclo de vida.**

AWC no será un agente.

AWC no incluirá un LLM.

AWC no decidirá cómo diseñar una arquitectura.

AWC no realizará code review.

AWC no sustituirá OpenClaw, Hermes ni otros agentes.

Será un **filesystem control plane for AI agents**.

---

# 2. Problema

Los agentes persistentes presentan varios problemas relacionados entre sí.

## 2.1 Desorganización

Un agente con permisos de filesystem puede crear archivos libremente.

Con el tiempo aparecen:

```text
plan.md
plan2.md
review-final.md
temp.txt
notes.md
research-old.md
```

sin estructura consistente.

---

## 2.2 Ausencia de lifecycle

Normalmente no existe información suficiente para responder:

* ¿este documento sigue activo?
* ¿era temporal?
* ¿está reemplazado por otro?
* ¿puede eliminarse?
* ¿pertenece a una tarea terminada?
* ¿debe archivarse?
* ¿es basura?
* ¿qué proyecto lo creó?

---

## 2.3 Planes monolíticos

Ante una funcionalidad compleja, un LLM puede producir:

```text
FEATURE_PLAN.md
```

con cientos o miles de líneas mezclando:

* frontend;
* backend;
* base de datos;
* decisiones;
* arquitectura;
* endpoints;
* testing;
* migraciones;
* riesgos;
* TODOs.

Eso dificulta ejecución incremental, revisión y recuperación de contexto.

---

## 2.4 Artefactos que desaparecen dentro del chat

Una investigación o code review puede haber requerido una cantidad considerable de razonamiento y posteriormente quedar enterrada en el historial conversacional.

AWC debe convertir esos resultados en artefactos persistentes cuando corresponda.

---

## 2.5 Pérdida de conocimiento operacional

El agente puede necesitar saber:

```text
Servidor de producción → SSH alias flyadd-prod
Base de datos → credencial flyadd-db
Repositorio → ruta /projects/flyadd
```

sin almacenar necesariamente los valores secretos.

---

## 2.6 Secretos dentro de memoria y Markdown

Una solución ingenua sería guardar:

```text
password: ...
privateKey: ...
apiKey: ...
```

dentro de `MEMORY.md` o archivos del workspace.

Esto es especialmente peligroso en un entorno donde el agente puede leer filesystem, ejecutar procesos y enviar output al contexto de conversación.

AWC almacenará **referencias a secretos**, nunca el secreto mismo.

---

# 3. Objetivo real de enforcement

AWC no asumirá que un LLM cumplirá perfectamente sus instrucciones.

Por tanto, el objetivo inicial NO será:

> El agente siempre utiliza AWC.

Eso sería imposible de garantizar mientras el runtime permita operaciones arbitrarias sobre filesystem.

La garantía buscada será:

> **Todo desorden, inconsistencia o artefacto no administrado debe ser detectable y recuperable.**

El sistema se diseña suponiendo que eventualmente ocurrirá esto:

```text
Agent
  │
  ├── usa AWC correctamente ───────────────┐
  │                                       │
  └── escribe fuera de protocolo          │
             │                            │
             ▼                            │
         Detection                        │
             │                            │
             ▼                            │
          Doctor                          │
             │                            │
             ▼                            │
      Reconciliation                      │
             │                            │
       ┌─────┴─────┐                      │
       ▼           ▼                      │
     Repair       Inbox                   │
       │           │                      │
       └─────┬─────┘                      │
             ▼                            │
      Managed workspace ◄─────────────────┘
```

---

# 4. Métrica principal

La métrica de éxito no será:

```text
100 % de llamadas pasan por awctl
```

sino:

```text
ningún archivo no administrado permanece
indefinidamente sin ser detectado
```

Para un entorno persistente podría establecerse como objetivo:

> **Todo archivo no administrado debe detectarse dentro de las siguientes 24 horas o durante el siguiente ciclo completo de diagnóstico.**

Otras métricas:

* porcentaje de artefactos importantes registrados;
* porcentaje de inconsistencias reconciliables;
* número de archivos huérfanos;
* número de secretos detectados en contenido persistente;
* operaciones destructivas ejecutadas sin plan: **0**;
* falsos positivos de cleanup;
* cumplimiento del protocolo por parte del agente;
* overhead de tokens introducido por AWC.

---

# 5. Principios de diseño

## 5.1 Deterministic over intelligent

Cuando una regla pueda resolverse determinísticamente, no se utilizará un LLM.

---

## 5.2 Preserve over delete

Cuando AWC no sepa qué hacer:

```text
inbox
archive
trash
```

antes que eliminación.

---

## 5.3 References over secrets

AWC recuerda:

```text
qué credencial usar
```

no:

```text
su valor
```

---

## 5.4 Structured over monolithic

Una unidad compleja de trabajo se divide en artefactos y work items relacionados.

---

## 5.5 Agent-agnostic by default

El núcleo no dependerá de:

```text
OpenClaw SDK
Hermes SDK
Claude Code SDK
Codex SDK
```

---

## 5.6 Machine-readable first

Toda operación debe poder consumirse de manera robusta por un agente.

---

## 5.7 Human-readable too

La misma herramienta debe ser agradable desde terminal.

---

## 5.8 Repairability over perfect prevention

AWC debe recuperarse de:

* procesos interrumpidos;
* archivos movidos manualmente;
* archivos eliminados;
* escritura fuera de protocolo;
* metadata parcialmente inconsistente.

---

# 6. Agnosticismo del agente

AWC no se implementará como plugin exclusivo de OpenClaw.

La arquitectura será:

```text
                    ┌──────────────────┐
                    │     AWC Core     │
                    │       Rust       │
                    └────────┬─────────┘
                             │
             ┌───────────────┼───────────────┐
             │                               │
             ▼                               ▼
           awctl                         AWC MCP
            CLI                           Server
             │                               │
             │                   ┌───────────┴──────────┐
             │                   ▼                      ▼
             │                OpenClaw               Hermes
             │
             ▼
           Human
```

Sobre ello habrá una tercera capa:

```text
AgentSkill portable
```

y opcionalmente:

```text
native adapters
```

para runtimes concretos.

---

# 7. Capas de integración

AWC tendrá tres niveles.

## Nivel 1 — CLI universal

Requisito:

> El agente puede ejecutar un proceso local.

Interfaz:

```bash
awctl ...
```

Ejemplo:

```bash
awctl artifact create \
  --project foodly \
  --type code-review \
  --title "PR 284" \
  --json
```

Este nivel debe funcionar incluso aunque el host no soporte MCP ni plugins.

---

# 8. Nivel 2 — Agent Skill + MCP

Ésta será la integración recomendada.

## Agent Skill

Indica:

* cuándo debe utilizarse AWC;
* qué artefactos deben persistirse;
* cuándo crear un plan;
* cuándo utilizar work items;
* cuándo ejecutar `doctor`;
* qué está prohibido almacenar;
* cómo actuar cuando AWC reporta inconsistencias.

## MCP

Proporciona herramientas tipadas:

```text
awc_context
awc_artifact_create
awc_artifact_archive
awc_plan_create
awc_work_create
awc_work_ready
awc_cleanup_scan
awc_cleanup_apply
awc_security_scan
```

Así el agente no tiene que construir comandos shell.

---

# 9. Compatibilidad con OpenClaw

OpenClaw actualmente separa conceptualmente tools, skills y plugins; sus skills son documentos `SKILL.md` que enseñan al agente **cómo y cuándo** utilizar herramientas. También carga skills desde el workspace, `.agents/skills` y ubicaciones personales, entre otras fuentes.

Esta separación coincide directamente con AWC:

```text
AgentSkill
    ↓
cuándo usar AWC

MCP / CLI
    ↓
cómo invocar AWC
```

OpenClaw también dispone de soporte MCP, lo que permite ejecutar AWC como servidor externo en vez de convertir el núcleo en un plugin propio del runtime.

---

# 10. Compatibilidad con Hermes

Hermes permite conectarse a servidores MCP externos y realiza descubrimiento de sus herramientas al iniciar. Su documentación recomienda MCP precisamente para acceder a herramientas externas sin implementar primero una herramienta nativa de Hermes.

Hermes también tiene un ecosistema de skills `SKILL.md` y puede instalar skills desde distintas fuentes, incluidos repositorios y URLs.

Además, Hermes soporta paquetes portables que pueden incluir simultáneamente:

```text
skills/
mcp.json
```

aunque actualmente su subconjunto portable se limita a MCP stdio y tiene límites propios de confianza y sandbox.

Por tanto:

```text
AWC Skill + AWC MCP stdio
```

es una base razonablemente portable entre OpenClaw y Hermes sin depender de los plugins nativos de ninguno.

---

# 11. Nivel 3 — Native adapters

Un native adapter será opcional.

Su responsabilidad NO será implementar lógica AWC.

Solo permitirá capacidades específicas del host:

* lifecycle;
* context injection;
* policy enforcement;
* tool interception;
* session hooks;
* startup checks.

Ejemplo:

```text
OpenClaw Adapter
       │
before_tool_call
       │
       ▼
AWC policy check
```

OpenClaw expone plugin hooks capaces de inspeccionar o modificar ejecuciones, tool calls y lifecycle de sesiones, incluyendo un `before_tool_call`.

El mismo patrón podrá adaptarse a Hermes mediante sus extensiones nativas cuando resulte necesario.

---

# 12. Regla de los adaptadores

Los plugins NO implementarán:

```text
ArtifactService
CleanupService
SQLite
PathPolicy
Reconciliation
SecretReference
```

Harán:

```text
Host event
    │
    ▼
AWC interface
    │
    ▼
decision
    │
    ▼
Host response
```

Así evitamos:

```text
TypeScript implementation
+
Python implementation
+
Rust implementation
```

de la misma regla.

---

# 13. ¿Por qué CLI y MCP simultáneamente?

MCP no reemplaza el CLI.

El CLI continúa siendo útil para:

* humanos;
* scripting;
* debugging;
* CI;
* hooks simples;
* recovery;
* hosts sin MCP;
* instalación;
* administración.

MCP es mejor para:

```text
agent → AWC
```

porque ofrece inputs estructurados.

CLI es mejor para:

```text
human/operator → AWC
```

y como fallback universal.

---

# 14. Distribución conceptual

```text
                     Agent Workspace Control
                              │
            ┌─────────────────┴─────────────────┐
            │                                   │
            ▼                                   ▼
         Rust Core                        Portable Skill
            │                                   │
      ┌─────┴─────┐                             │
      ▼           ▼                             │
   awctl       AWC MCP                          │
      │           │                             │
      │     ┌─────┴────────────┐                │
      │     ▼                  ▼                │
      │  OpenClaw           Hermes ◄────────────┘
      │     │                  │
      │     ▼                  ▼
      │ optional adapter  optional adapter
      │
      ▼
    Human
```

---

# 15. Relación con proyectos existentes

AWC no aparece en un vacío.

Existen soluciones que atacan partes del problema.

---

# 16. Beads

Beads es un issue tracker orientado a agentes que utiliza un grafo persistente de dependencias para reemplazar planes Markdown desordenados y mantener trabajo de larga duración.

Su dominio principal es:

```text
work tracking
dependency graph
ready work
claim
close
```

AWC no debería competir construyendo un issue tracker sofisticado.

AWC tendrá únicamente gestión suficiente de work items para organizar artefactos.

Una integración futura con Beads es preferible a duplicar todo su dominio.

---

# 17. OpenSpec

OpenSpec busca evitar que los requirements importantes vivan exclusivamente dentro del historial de chat y organiza cada cambio en una carpeta con artefactos como proposal, specs, design y tasks.

AWC es más general:

```text
OpenSpec
    ↓
software change artifacts

AWC
    ↓
workspace-wide artifact lifecycle
```

AWC podría eventualmente adoptar/indexar estructuras OpenSpec.

---

# 18. Spec Kit

GitHub Spec Kit es un toolkit de Spec-Driven Development diseñado para trabajar con distintos coding agents y proporcionar un proceso estructurado de desarrollo.

AWC no impondrá una metodología de desarrollo.

Un proyecto podrá utilizar:

```text
Spec Kit
OpenSpec
Beads
Markdown propio
```

y AWC gobernará sus efectos sobre workspace.

---

# 19. Diferenciación

La propuesta específica de AWC será:

> **Gobernanza determinista del lifecycle del workspace de un agente.**

Áreas principales:

| Área                     | AWC |
| ------------------------ | --- |
| Artifact registry        | Sí  |
| Workspace hygiene        | Sí  |
| Unmanaged file detection | Sí  |
| Reconciliation           | Sí  |
| Trash lifecycle          | Sí  |
| Temporary lifecycle      | Sí  |
| Cleanup plans            | Sí  |
| Secret references        | Sí  |
| Agent context            | Sí  |
| Portable AgentSkill      | Sí  |
| MCP                      | Sí  |
| Task graph avanzado      | No  |
| LLM planning             | No  |
| Spec methodology         | No  |
| Secret vault             | No  |

---

# 20. Arquitectura del software

El núcleo será Rust.

```text
┌────────────────────────────────────────────┐
│                  awctl                     │
│                    │                       │
│                    ▼                       │
│               Application                  │
│                    │                       │
│       ┌────────────┼─────────────┐         │
│       ▼            ▼             ▼         │
│   Artifacts       Work        Hygiene      │
│       │            │             │         │
│       └────────────┼─────────────┘         │
│                    ▼                       │
│                  Core                      │
│                    │                       │
│            ┌───────┴───────┐               │
│            ▼               ▼               │
│          SQLite        Filesystem          │
└────────────────────────────────────────────┘
```

El MCP server utiliza el mismo core:

```text
MCP request
    │
    ▼
awc-mcp
    │
    ▼
awc-core
```

---

# 21. Rust workspace

La estructura física inicial recomendada:

```text
agent-workspace-control/
├── Cargo.toml
├── crates/
│   ├── awc-core/
│   │   └── src/
│   │
│   ├── awctl/
│   │   └── src/
│   │
│   └── awc-mcp/
│       └── src/
│
├── skills/
│   └── agent-workspace-control/
│       ├── SKILL.md
│       └── references/
│
├── integrations/
│   ├── openclaw/
│   └── hermes/
│
├── migrations/
│
└── tests/
```

Sin embargo, `awc-mcp` puede incorporarse después de estabilizar el CLI.

---

# 22. Dependencias entre crates

```text
            awc-core
              ▲   ▲
              │   │
              │   │
           awctl  awc-mcp
```

`awc-core` NO dependerá de:

```text
clap
rmcp
OpenClaw
Hermes
Tokio
```

---

# 23. Async

El core será síncrono.

El CLI también.

No se introducirá Tokio hasta implementar MCP.

El SDK oficial Rust de MCP (`rmcp`) utiliza Tokio como runtime async.

Por tanto:

```text
awc-core
    ↓
sync

awctl
    ↓
sync

awc-mcp
    ↓
Tokio + rmcp
```

Esto permite aprender Rust incrementalmente sin contaminar todo el dominio con async.

---

# 24. Dependencias Rust iniciales

Para la primera versión:

```toml
clap
serde
serde_json
toml
rusqlite
uuid
thiserror
sha2
```

Posteriormente:

```toml
rmcp
tokio
```

`clap` será utilizado para modelar subcomandos y argumentos del CLI mediante tipos Rust.

`rusqlite` será utilizado como wrapper SQLite; su feature `bundled` permite distribuir la biblioteca SQLite junto a la aplicación en vez de depender obligatoriamente de una instalación del sistema.

---

# 25. Workspace administrado

Un workspace típico:

```text
workspace/
├── AGENTS.md
├── SOUL.md
├── USER.md
├── MEMORY.md
├── memory/
├── skills/
│
├── artifacts/
│   ├── plans/
│   ├── reviews/
│   ├── research/
│   ├── reports/
│   ├── decisions/
│   ├── handoffs/
│   └── archive/
│
├── inbox/
├── tmp/
├── trash/
│
└── .awc/
    ├── state.db
    ├── config.toml
    └── runtime/
```

Se utilizará:

```text
.awc/
```

para estado interno del producto.

`awctl` es el nombre del ejecutable.

`AWC` es el producto.

---

# 26. Ownership de paths

No todo archivo pertenece a AWC.

Se definirá:

```rust
enum PathOwnership {
    AwcManaged,
    AgentRuntimeManaged,
    UserManaged,
    Ignored,
    Unmanaged,
}
```

Ejemplos OpenClaw:

```text
.awc/**           → AwcManaged
artifacts/**      → AwcManaged
inbox/**          → AwcManaged

AGENTS.md         → AgentRuntimeManaged
SOUL.md           → AgentRuntimeManaged
MEMORY.md         → AgentRuntimeManaged
memory/**         → AgentRuntimeManaged
skills/**         → AgentRuntimeManaged

.git/**           → Ignored
target/**         → Ignored

docs/**           → UserManaged

random-plan.md    → Unmanaged
```

---

# 27. Regla crítica

`AgentRuntimeManaged` significa:

> AWC conoce este path pero no controla su contenido ni lifecycle.

Esto evita que cleanup considere basura archivos legítimos administrados por OpenClaw, Hermes u otro runtime.

---

# 28. Fuente de verdad

Habrá una separación explícita:

```text
SQLite
    =
identity
metadata
status
relationships
path
hash
lifecycle

Markdown/files
    =
content
```

El agente continúa siendo libre de escribir contenido.

AWC gobierna identidad y lifecycle.

---

# 29. Entidades del MVP

El primer esquema debe ser deliberadamente pequeño.

Inicialmente:

```text
Project
Artifact
AuditEvent
```

Posteriormente:

```text
WorkItem
WorkDependency
SecretReference
```

---

# 30. Project

Representa un contexto lógico.

```rust
struct Project {
    id: ProjectId,
    slug: String,
    name: String,
    root_path: Option<PathBuf>,
    status: ProjectStatus,
    created_at: Timestamp,
    updated_at: Timestamp,
}
```

Ejemplos:

```text
foodly
flyadd
siamp
university
personal
```

Un proyecto no tiene que corresponder 1:1 a un repositorio Git.

---

# 31. Artifact

Entidad central.

```rust
struct Artifact {
    id: ArtifactId,
    project_id: Option<ProjectId>,
    artifact_type: ArtifactType,
    title: String,
    path: PathBuf,
    status: ArtifactStatus,

    content_hash: Option<ContentHash>,
    content_size: Option<u64>,
    last_seen_at: Option<Timestamp>,

    created_at: Timestamp,
    updated_at: Timestamp,
}
```

---

# 32. ArtifactType

```rust
enum ArtifactType {
    Plan,
    WorkDocument,
    CodeReview,
    Research,
    Report,
    Decision,
    Handoff,
    Documentation,
    Other,
}
```

---

# 33. ArtifactStatus

```rust
enum ArtifactStatus {
    Active,
    Completed,
    Archived,
    Trashed,
}
```

Las transiciones serán explícitas.

---

# 34. Identidad de archivos

AWC guardará:

```text
path
SHA-256
size
last_seen_at
```

Esto permite reconciliar movimientos externos.

Ejemplo:

```text
DB:

ART-A
path = artifacts/reviews/foo.md
hash = ABC
```

El archivo desaparece.

AWC encuentra:

```text
inbox/foo-renamed.md
hash = ABC
```

Puede inferir:

```text
probable move
```

---

# 35. Reconciliación segura

Un mismo hash no implica necesariamente movimiento.

Puede existir una copia.

Regla:

```text
expected path missing
        │
        ▼
search hash candidates
        │
   ┌────┴────┐
   │         │
1 candidate  N candidates
   │         │
   ▼         ▼
high       ambiguous
confidence
```

AWC no corregirá automáticamente coincidencias ambiguas.

---

# 36. IDs

Se utilizará UUIDv7 internamente.

Ejemplo:

```text
019c4f86-...
```

Los tipos usarán newtypes:

```rust
struct ArtifactId(Uuid);
struct ProjectId(Uuid);
struct WorkItemId(Uuid);
```

La CLI podrá mostrar prefijos:

```text
ART-019c4f86
```

---

# 37. Resolución de prefijos

Cuando el usuario escriba:

```bash
awctl artifact show 019c4f
```

AWC realizará:

```text
0 matches
    → NOT_FOUND

1 match
    → resolved

>1 matches
    → AMBIGUOUS_ID
```

Nunca se asumirá que un prefijo truncado es globalmente único.

---

# 38. WorkItem

No habrá conceptos separados `Task` e `Issue`.

Ambos representan esencialmente una unidad ejecutable.

Se utilizará:

```rust
struct WorkItem {
    id: WorkItemId,
    project_id: ProjectId,
    parent_artifact_id: Option<ArtifactId>,
    artifact_id: Option<ArtifactId>,

    title: String,
    kind: WorkItemKind,
    status: WorkItemStatus,

    created_at: Timestamp,
    updated_at: Timestamp,
}
```

---

# 39. Work item como unidad + artefacto

```text
WORK-123
   │
   ├── metadata
   ├── status
   ├── dependencies
   │
   └── artifact
         │
         ▼
BE-003-notification-api.md
```

Un solo ID representa la unidad de trabajo.

No:

```text
ISSUE-123
TASK-456
```

para el mismo concepto.

---

# 40. WorkItemKind

Inicialmente:

```rust
enum WorkItemKind {
    Backend,
    Frontend,
    Database,
    Infrastructure,
    QA,
    Documentation,
    Research,
    Generic,
}
```

Estos valores describen área, no metodología.

---

# 41. WorkItemStatus

```rust
enum WorkItemStatus {
    Pending,
    InProgress,
    Blocked,
    Completed,
    Cancelled,
}
```

---

# 42. Dependencias

```text
WORK-B requires WORK-A
```

almacenado como:

```text
WORK-A → WORK-B
```

Antes de agregar una dependencia se validará que no cree ciclos.

---

# 43. Plan

Un Plan será un artefacto compuesto.

No un Markdown gigante.

Ejemplo:

```text
artifacts/plans/foodly/notifications/
├── README.md
├── architecture.md
└── work/
    ├── BE-001-domain.md
    ├── BE-002-persistence.md
    ├── BE-003-api.md
    ├── FE-001-client.md
    ├── FE-002-ui.md
    └── QA-001-integration.md
```

---

# 44. README del plan

Debe ser intencionalmente pequeño:

```markdown
# Notifications

## Goal

## Scope

## Architecture summary

## Work items

## Execution order

## Risks

## Decisions
```

No debe repetir todos los detalles de los work items.

---

# 45. Plantilla de work item

```markdown
# BE-003 — Notification API

## Goal

## Depends on

## Scope

## Business rules

## Acceptance criteria

## Tests

## Out of scope
```

La skill deberá enseñar al agente:

> Si dos partes pueden implementarse y validarse independientemente, deberían ser work items separados.

---

# 46. Code reviews

Un code review debe convertirse en artefacto persistente.

Ejemplo:

```text
artifacts/reviews/flyadd/
└── 2026-08-08-pr-284.md
```

Plantilla:

```markdown
# Code Review

## Context

## Summary

## Blocking findings

## High severity

## Medium severity

## Low severity

## Missing tests

## Architectural observations

## Verdict
```

Incluso:

```text
No findings
```

es un resultado persistible.

---

# 47. Temporary files

`tmp/` tendrá semántica explícita.

Nada debería quedarse ahí indefinidamente.

Un temporal tendrá como mínimo asociación lógica con:

```text
project
work item
artifact
session
```

cuando esté disponible.

Ejemplo:

```text
tmp/WORK-123/
├── api-response.json
└── extracted-schema.txt
```

---

# 48. Lifecycle temporal

```text
temporary
    │
    ▼
active
    │
work completed
    ▼
expired candidate
    │
    ▼
cleanup
```

No se eliminará automáticamente inmediatamente.

---

# 49. Trash

La eliminación normal será:

```text
ACTIVE
   │
   ▼
TRASH
   │
retention
   ▼
PURGE
```

No:

```text
ACTIVE → rm
```

---

# 50. Cleanup

Cleanup será una operación explícitamente planificada.

```text
scan
  │
  ▼
plan
  │
  ▼
review
  │
  ▼
apply
```

---

# 51. Cleanup Scan

Ejemplo:

```bash
awctl cleanup scan --json
```

Resultado:

```json
{
  "schemaVersion": 1,
  "ok": true,
  "data": {
    "planId": "CLN-019c...",
    "actions": [
      {
        "type": "move_to_inbox",
        "path": "plan-old.md",
        "reason": "UNMANAGED_ROOT_FILE"
      }
    ]
  }
}
```

---

# 52. Cleanup plans

No serán inicialmente una entidad del dominio ni una tabla.

Se guardarán como estado técnico:

```text
.awc/runtime/cleanup/
└── CLN-019c....json
```

Después de aplicarse:

```text
audit
+
plan removal
```

---

# 53. Protección contra planes obsoletos

El cleanup plan incluirá:

```text
plan hash
workspace fingerprint
selected paths
expected metadata
```

Antes del `apply`, AWC comprobará que las precondiciones continúan siendo válidas.

Si el workspace cambió:

```text
STALE_CLEANUP_PLAN
```

y la operación se rechaza.

---

# 54. Dry run

Toda operación destructiva masiva deberá soportar:

```bash
--dry-run
```

Ejemplo:

```bash
awctl cleanup apply CLN-ID --dry-run
```

---

# 55. Adopt

`awctl init` no es suficiente para usuarios existentes.

AWC necesita onboarding brownfield.

El flujo será:

```bash
awctl adopt scan
```

para inspeccionar un workspace existente.

---

# 56. Adopt Scan

Clasificará:

```text
Known runtime files
Managed candidates
Temporary candidates
Sensitive candidates
Unknown
Ignored
```

Ejemplo:

```text
Recognized runtime files
✓ AGENTS.md
✓ MEMORY.md
✓ memory/

Possible artifacts
! notification-plan.md
! pr-review.md

Temporary candidates
! result.json

Unknown
! notes-old.md
```

No modificará nada.

---

# 57. Adopt Plan

```bash
awctl adopt plan
```

produce:

```text
adopt-existing-plan.md
    → register as Plan?

review-pr-13.md
    → register as CodeReview?

random.md
    → move to inbox?
```

---

# 58. Adopt Apply

```bash
awctl adopt apply ADOPT-ID
```

ejecuta únicamente las acciones explícitas del plan.

---

# 59. Filosofía de clasificación

AWC no hará NLP complejo en V1.

Utilizará únicamente señales deterministas:

* ubicación;
* nombre;
* extensión;
* firmas conocidas;
* hashes;
* frontmatter;
* configuración;
* directorios conocidos.

Una heurística puede generar una sugerencia.

Nunca convertir incertidumbre en eliminación.

---

# 60. Inbox

`inbox/` representa:

> Archivo conservado cuya clasificación definitiva todavía no conocemos.

Esto es diferente de:

```text
trash/
```

Inbox:

```text
unknown
```

Trash:

```text
known unwanted
```

---

# 61. Doctor

`awctl doctor` será uno de los comandos centrales.

Categorías:

```text
DATABASE
FILESYSTEM
ARTIFACTS
WORK
RUNTIME
HYGIENE
SECURITY
GIT
```

---

# 62. Doctor Quick

```bash
awctl doctor --quick
```

debe ser barato.

Incluye:

* apertura de DB;
* versión schema;
* parseo de configuración;
* existencia de directorios esenciales;
* paths registrados;
* integración del agente;
* root pollution obvia;
* cleanup plan sanity.

No incluye:

* hashing completo;
* content scanning;
* traversals profundos;
* secret scanning completo.

---

# 63. Doctor completo

```bash
awctl doctor
```

sí puede:

* recorrer workspace;
* calcular hashes;
* buscar huérfanos;
* detectar duplicados;
* reconciliar movimientos;
* validar dependencies;
* inspeccionar temporales;
* comprobar Git;
* buscar secretos.

---

# 64. Reconciliation

Ejemplo:

```text
! ART-91

expected:
artifacts/reviews/foo.md

not found.

possible match:
inbox/foo-renamed.md

hash: identical
size: identical
confidence: high
```

AWC propondrá:

```bash
awctl artifact relink ART-91 inbox/foo-renamed.md
```

---

# 65. SecretReference

AWC no almacenará valores.

Modelo conceptual:

```rust
struct SecretReference {
    id: SecretRefId,
    project_id: Option<ProjectId>,
    name: String,
    provider: String,
    external_id: String,
    purpose: Option<String>,
}
```

No existirá:

```rust
value: String
```

---

# 66. Secret providers

AWC solamente necesitará conocer referencias como:

```text
env:OPENAI_API_KEY
ssh:flyadd-prod
openclaw:flyadd-db
1password:item-id
pass:prod/database
```

sin resolverlas necesariamente.

---

# 67. No `secret resolve`

No habrá:

```bash
awctl secret resolve
```

en el MVP.

Eso evitará que un secreto termine accidentalmente en:

```text
stdout
tool output
conversation transcript
log
```

Habrá:

```bash
awctl secret check flyadd-db
```

que devolverá únicamente:

```json
{
  "exists": true,
  "resolvable": true
}
```

---

# 68. SSH

Para SSH:

```text
connection:
  id: flyadd-prod
  alias: flyadd-prod
```

y el agente usa:

```bash
ssh flyadd-prod
```

AWC no necesita almacenar la private key.

---

# 69. Security Scan

La detección de secretos debe realizarse **después de que el agente haya escrito contenido**, no solamente al crear el archivo.

Comando:

```bash
awctl security scan
```

Puede detectar patrones obvios como:

```text
-----BEGIN OPENSSH PRIVATE KEY-----
password=
api_key=
Authorization: Bearer
```

sin pretender ser una solución DLP completa.

---

# 70. Security scan integrado

También podrá ejecutarse durante:

```text
doctor
cleanup scan
artifact archive
```

dependiendo de configuración.

---

# 71. Git

Git será recomendado pero no obligatorio.

`doctor` detectará:

```text
Git repository: yes/no
```

Git ofrece una segunda capa de recuperación.

AWC no realizará commits automáticamente en el MVP.

---

# 72. Artifact operations

Regla operacional deseada:

```text
CREATE    → AWC
MOVE      → AWC
ARCHIVE   → AWC
TRASH     → AWC
RESTORE   → AWC
LIFECYCLE → AWC

EDIT      → Agent
```

El agente puede modificar el contenido directamente después de que AWC cree el artefacto.

---

# 73. Límites del enforcement

Una Skill no es una barrera de seguridad.

Si un agente dispone de shell completo, potencialmente puede ignorar AWC.

Por eso el sistema tendrá:

```text
behavioral enforcement
    ↓
Skill

detection + repair
    ↓
Doctor

strong enforcement
    ↓
optional native adapter
```

---

# 74. Native adapter futuro

Cuando se necesite enforcement fuerte:

```text
tool call
   │
   ▼
runtime hook
   │
   ▼
awc policy check
   │
   ├── allow
   ├── warn
   ├── require approval
   └── deny
```

En OpenClaw, su plugin API ya contempla hooks in-process sobre tool calls y lifecycle, por lo que este tipo de integración es viable sin introducir esas APIs dentro del core AWC.

---

# 75. Agent Skill

La Skill es una parte del producto, no documentación secundaria.

Ubicación en el repositorio:

```text
skills/
└── agent-workspace-control/
    ├── SKILL.md
    └── references/
```

---

# 76. Responsabilidad de la Skill

Debe enseñar:

### Antes de crear contenido persistente

Clasificar:

```text
Plan
CodeReview
Research
Report
Decision
Handoff
Documentation
Temporary
```

### Para persistent artifacts

Usar AWC.

### Para modificaciones de contenido

Editar directamente el archivo ya registrado.

### Para planificación compleja

Crear plan + work items.

### Para secretos

Registrar únicamente referencias.

### Antes de concluir trabajos complejos

Consultar AWC y validar estado.

---

# 77. MCP-first con CLI fallback

La Skill incluirá una regla conceptual:

```text
If AWC MCP tools are available:
    prefer MCP.

Else if awctl exists:
    use awctl --json.

Else:
    report that AWC is unavailable.
```

Esto hace a la Skill portable.

---

# 78. Context

`awctl context` estará específicamente diseñado para agentes.

```bash
awctl context --json
```

No debe devolver todo el workspace.

Solo información de alto valor:

```json
{
  "schemaVersion": 1,
  "workspace": {
    "health": "warning"
  },
  "activeWork": [],
  "blockedWork": [],
  "recentArtifacts": [],
  "warnings": []
}
```

---

# 79. Context budget

El comando soportará límites:

```bash
awctl context --max-items 10
```

y eventualmente:

```bash
awctl context --budget compact
```

El objetivo es minimizar tokens.

---

# 80. Diferencia Status vs Context

## `awctl status`

Optimizado para persona.

Más descriptivo.

## `awctl context`

Optimizado para agente.

Más pequeño.

Estructurado.

Solo información inmediatamente accionable.

---

# 81. Capabilities

```bash
awctl capabilities --json
```

Ejemplo:

```json
{
  "schemaVersion": 1,
  "capabilities": {
    "artifacts": true,
    "workItems": true,
    "cleanup": true,
    "secretReferences": false,
    "mcp": true
  }
}
```

La Skill puede descubrir funciones sin asumir versión.

---

# 82. CLI completo propuesto

```text
awctl
├── init
├── adopt
│   ├── scan
│   ├── plan
│   └── apply
│
├── status
├── context
├── capabilities
├── doctor
│
├── project
│   ├── add
│   ├── list
│   ├── show
│   └── archive
│
├── artifact
│   ├── create
│   ├── show
│   ├── list
│   ├── relink
│   ├── archive
│   ├── trash
│   └── restore
│
├── plan
│   ├── create
│   ├── show
│   └── validate
│
├── work
│   ├── create
│   ├── show
│   ├── list
│   ├── ready
│   ├── start
│   ├── block
│   ├── close
│   ├── cancel
│   └── dependency
│
├── review
│   └── create
│
├── tmp
│   ├── create
│   ├── list
│   └── expire
│
├── cleanup
│   ├── scan
│   ├── show
│   └── apply
│
├── trash
│   ├── list
│   ├── restore
│   └── purge
│
├── security
│   └── scan
│
├── secret
│   ├── register
│   ├── list
│   └── check
│
└── integration
    ├── list
    ├── detect
    ├── install
    ├── status
    └── doctor
```

---

# 83. Integrations

En vez de:

```bash
awctl openclaw install
```

se utilizará:

```bash
awctl integration install openclaw
awctl integration install hermes
```

Así el modelo central no privilegia un agente.

---

# 84. MCP tools

El MCP server no debe exponer cada comando CLI ciegamente.

Tendrá una superficie pequeña y semántica.

Primera versión:

```text
awc_context
awc_status

awc_artifact_create
awc_artifact_show
awc_artifact_archive

awc_plan_create
awc_plan_validate

awc_work_create
awc_work_ready
awc_work_start
awc_work_close

awc_cleanup_scan
awc_cleanup_apply

awc_security_scan
```

---

# 85. Operaciones destructivas por MCP

No se expondrá inicialmente:

```text
trash_purge --force
```

como herramienta fácil para agentes.

Operaciones de mayor riesgo pueden requerir:

* aprobación humana;
* CLI manual;
* policy específica.

---

# 86. MCP transport

La primera implementación será:

```text
stdio
```

porque:

* es local;
* evita abrir puertos;
* simplifica instalación;
* encaja con el modelo portable;
* Hermes actualmente soporta stdio en su subset portable de Agent Plugins.

Posteriormente podría evaluarse Streamable HTTP.

---

# 87. Contrato JSON del CLI

Todo comando agent-friendly soportará:

```bash
--json
```

Respuesta:

```json
{
  "schemaVersion": 1,
  "ok": true,
  "data": {}
}
```

Error:

```json
{
  "schemaVersion": 1,
  "ok": false,
  "error": {
    "code": "ARTIFACT_NOT_FOUND",
    "message": "Artifact was not found"
  }
}
```

---

# 88. Stdout y stderr

Regla:

```text
stdout = result
stderr = diagnostics
```

Si `--json` está activo, stdout contiene exclusivamente JSON válido.

Nunca:

```text
Opening database...
Migration complete...
{"ok": true}
```

---

# 89. Exit codes

Propuesta:

```text
0  SUCCESS
2  INVALID_ARGUMENT
3  CONFIGURATION_ERROR
4  POLICY_VIOLATION
5  CONFLICT
6  NOT_FOUND
7  STORAGE_ERROR
8  UNSAFE_OPERATION_REFUSED
9  INTEGRATION_ERROR
```

El consumidor puede utilizar:

```text
exit status + error.code
```

---

# 90. Error model Rust

```rust
enum AwcError {
    NotFound(...),
    Conflict(...),
    PolicyViolation(...),
    Storage(...),
    Filesystem(...),
    InvalidState(...),
    UnsafeOperation(...),
    Integration(...),
}
```

`thiserror` será suficiente inicialmente.

---

# 91. SQLite

SQLite será la fuente autoritativa de metadata.

Tablas iniciales:

```text
schema_migrations
projects
artifacts
audit_events
```

Posteriormente:

```text
work_items
work_dependencies
secret_references
```

---

# 92. Artifacts schema conceptual

```sql
artifacts
---------
id
project_id
artifact_type
title
path
status
content_hash
content_size
last_seen_at
created_at
updated_at
```

---

# 93. Audit

Toda mutación relevante:

```text
project.created
artifact.created
artifact.relinked
artifact.archived
artifact.trashed
artifact.restored
work.created
work.started
work.completed
cleanup.applied
secret_reference.created
```

---

# 94. Auditoría y secretos

Nunca registrar:

```text
password
token
private key
Authorization header
secret value
```

Sí registrar:

```text
secret_ref_id
provider
operation
```

---

# 95. SQLite + filesystem

No existe una transacción ACID conjunta entre ambos.

Por eso se utilizarán operaciones compensatorias.

Ejemplo de artifact create:

```text
determine path
      │
      ▼
write temp
      │
      ▼
begin DB transaction
      │
      ▼
insert metadata
      │
      ▼
rename temp → final
      │
      ▼
commit
```

Ante error:

```text
rollback
cleanup temp
```

---

# 96. Crash recovery

A pesar de las compensaciones, un proceso puede morir en un punto crítico.

Por eso `doctor` debe detectar:

```text
metadata without file
file without metadata
temporary transaction residue
```

La reparación forma parte del diseño, no es una excepción rara.

---

# 97. Atomic filesystem operations

Cuando sea viable:

```text
write temporary
fsync if needed
atomic rename
```

Las garantías exactas dependerán del filesystem y plataforma.

Linux será la primera plataforma en la que esas garantías se validarán rigurosamente.

---

# 98. Platform support

## Tier 1

```text
Linux
```

## Tier 2

```text
macOS
```

## Experimental

```text
Windows
```

No se declarará Windows stable hasta probar:

* rename behavior;
* locking;
* paths;
* symlinks;
* permissions;
* deletion semantics.

---

# 99. Config

```toml
version = 1

[workspace]
artifacts = "artifacts"
inbox = "inbox"
temporary = "tmp"
trash = "trash"

[cleanup]
temporary_retention_days = 7
trash_retention_days = 30
auto_delete = false

[artifacts]
require_project = false
embed_id = false

[doctor]
secret_scan = true

[git]
recommended = true
required = false
```

---

# 100. Policy engine

No se construirá un lenguaje genérico de políticas.

Inicialmente serán reglas Rust:

```rust
enum PolicyViolation {
    RootArtifactForbidden,
    ProtectedPath,
    PathEscapesWorkspace,
    UnsafeDelete,
    SecretPersistenceRisk,
    DependencyCycle,
}
```

Solo se creará una abstracción más compleja si aparecen casos reales.

---

# 101. Path safety

Toda ruta proporcionada externamente deberá:

1. resolverse respecto al workspace;
2. normalizarse;
3. comprobar que no escapa del root permitido;
4. validar symlinks cuando corresponda;
5. aplicar ownership/policy.

Invariante:

> Ninguna operación administrada puede escapar silenciosamente del workspace.

---

# 102. Symlinks

Los symlinks requieren política explícita.

Default:

```text
do not follow external symlink targets
```

excepto configuración explícita.

Esto evita que un artefacto aparentemente local termine modificando:

```text
~/.ssh/
etc/
otro proyecto
```

---

# 103. Skill protection

La Skill principal de AWC debe considerarse infraestructura.

No debería ser editable libremente por el propio agente.

Idealmente:

```text
AWC-installed skill
    ↓
managed/read-only
```

y las modificaciones pasan por:

```text
upgrade
installer
operator approval
```

No queremos:

```text
Agent decides rule is inconvenient
           ↓
edits SKILL.md
           ↓
removes restriction
```

---

# 104. Self-improvement vs policy

El agente puede crear skills propias.

Pero:

```text
AWC policy skill
```

debe estar separada de:

```text
agent-generated skill
```

---

# 105. Integration detection

```bash
awctl integration detect
```

podría informar:

```text
Detected runtimes:

OpenClaw
  ✓ configuration found
  ✓ workspace recognized
  ✓ MCP available
  ○ native adapter not installed

Hermes
  ✓ installation found
  ✓ MCP available
  ○ portable skill not installed
```

---

# 106. Integration installation

El instalador no debe modificar silenciosamente configuración importante.

Modelo:

```text
detect
  ↓
plan
  ↓
show changes
  ↓
apply
```

Idealmente reutilizando el mismo patrón de operaciones seguras de AWC.

---

# 107. OpenClaw integration mínima

Debe instalar/configurar:

```text
Agent Skill
MCP stdio server
```

No requiere plugin nativo.

OpenClaw ya puede cargar skills de workspace o personales y utilizar MCP como capacidad separada.

---

# 108. Hermes integration mínima

Debe instalar/configurar:

```text
Agent Skill
MCP stdio server
```

Hermes soporta skills instalables y MCP externo; además sus paquetes portables pueden contener skill + MCP stdio juntos si más adelante conviene distribuir AWC de esa forma.

---

# 109. Native adapters: cuándo justificarlos

No se crearán porque “quedan bonitos”.

Solo si resolvemos una necesidad que Skill + MCP no puedan resolver.

Ejemplos:

```text
pre-write enforcement
tool-call blocking
automatic session context injection
host-specific approval UI
runtime-native lifecycle
```

---

# 110. Testing

Habrá varios niveles.

## Unit

```text
state transitions
path safety
hash matching
dependency cycles
policy evaluation
cleanup rules
```

## SQLite

Con DB temporal/in-memory.

## Filesystem

Con workspace temporal.

## CLI integration

Ejecutando el binario real.

## MCP contract

Invocando tools del servidor.

## Agent behavior

Sesiones reales con distintos agentes.

---

# 111. Tests conductuales

AWC necesita pruebas que no son unit tests tradicionales.

Escenario:

> Usuario solicita plan grande frontend + backend.

Expected:

```text
create plan
decompose into work items
write concise artifacts
validate plan
```

No:

```text
one huge markdown
```

---

# 112. Code review scenario

Input:

> Review this PR.

Expected:

```text
create CodeReview artifact
perform review
persist findings
return conversational summary
```

---

# 113. Secret scenario

Input contiene una credencial.

Expected:

```text
do not copy credential into persistent Markdown
register reference when possible
warn if secret scanner detects persistence
```

---

# 114. Disorder scenario

Agent manually creates:

```text
workspace/random-plan.md
```

Expected:

```text
doctor detects unmanaged root artifact
cleanup/adopt proposes safe classification
nothing deleted automatically
```

---

# 115. Move scenario

Agent manually moves:

```text
artifacts/reviews/foo.md
```

Expected:

```text
doctor notices missing registered path
finds unique hash candidate
proposes relink
```

---

# 116. Invariantes

Estas propiedades tendrán prioridad alta:

1. Ningún path administrado escapa del workspace.
2. Ningún cleanup destructivo se ejecuta desde un plan obsoleto.
3. Ningún archivo protegido se elimina automáticamente.
4. Ningún secreto raw se persiste en la tabla de referencias.
5. Ningún dependency cycle es aceptado.
6. JSON mode siempre produce JSON válido.
7. Un fallo de mutación no puede ignorarse silenciosamente.
8. Todo registro huérfano puede detectarse.
9. Todo archivo no administrado visible para scan puede clasificarse.
10. Purge es irreversible y requiere una intención clara.

---

# 117. Roadmap

## v0.1 — Foundation

Objetivo:

> Construir un CLI Rust pequeño y confiable.

Incluye:

```text
Cargo workspace
awc-core
awctl

init
status
doctor --quick

workspace discovery
config
SQLite migrations
Project
Artifact
AuditEvent
JSON contract
error model
```

---

# 118. v0.2 — Artifact Governance + Adopt

Incluye:

```text
project add/list/show

artifact create
artifact show
artifact list
artifact archive
artifact trash
artifact restore
artifact relink

path ownership
protected paths
hash + size

adopt scan
adopt plan
adopt apply
```

Al terminar esta fase AWC debe poder organizar un workspace existente.

---

# 119. v0.3 — Portable Agent Integration

Incluye:

```text
Agent Skill
context
capabilities

integration detect
integration install
integration status

OpenClaw skill
Hermes skill

real-world agent tests
```

Inicialmente puede utilizar CLI como backend.

La prioridad aquí es validar la **Skill**.

---

# 120. v0.4 — MCP

Añadir:

```text
awc-mcp
rmcp
Tokio

stdio transport

artifact tools
context tools
plan/work tools básicos
```

El SDK oficial Rust permite implementar MCP sin abandonar Rust, manteniendo async aislado dentro del crate MCP.

---

# 121. v0.5 — Hygiene & Reconciliation

Incluye:

```text
full doctor
hash reconciliation
cleanup scan
cleanup apply
tmp lifecycle
security scan
Git awareness
```

Aquí AWC comienza a cumplir su garantía principal:

> El desorden es detectable y recuperable.

---

# 122. v0.6 — Structured Planning

Añadir:

```text
WorkItem
WorkDependency

plan create
plan validate

work create
work list
work ready
work start
work block
work close
work graph
```

Se mantiene deliberadamente más pequeño que Beads.

---

# 123. v0.7 — Secret References

Incluye:

```text
SecretReference

secret register
secret list
secret check

SSH aliases
provider metadata
runtime-specific references
```

No vault.

No raw secret storage.

---

# 124. v0.8 — Native adapters experimentales

Solo después de observar limitaciones reales:

```text
OpenClaw adapter
Hermes adapter
```

Posibles capacidades:

```text
pre-tool-call policy
session context injection
automatic doctor quick
runtime lifecycle
```

---

# 125. v0.9 — Interoperability

Evaluar:

```text
Beads provider
OpenSpec adoption
Spec Kit artifacts
external WorkProvider
external ArtifactProvider
```

No crear traits prematuramente.

Solo cuando exista realmente un segundo implementation provider.

---

# 126. v1.0

Requisitos:

```text
Linux stable
macOS supported

SQLite migrations stable
JSON API stable

cleanup safe
reconciliation reliable
adoption reliable

Skill validated with real agents

OpenClaw working
Hermes working

MCP stable

no known raw-secret persistence paths
```

---

# 127. Qué NO entra en v1

```text
cloud service
multi-user server
web dashboard
vector DB
semantic search
embedded LLM
autonomous planner
password vault
complex issue tracker
Git hosting
distributed synchronization
```

---

# 128. Posible Beads integration

Si el WorkItem domain crece demasiado:

```toml
[work]
provider = "beads"
```

Conceptualmente:

```text
AWC
 │
 ├── artifacts
 ├── lifecycle
 ├── hygiene
 │
 └── work provider
        │
        ├── native
        └── beads
```

Beads ya está especializado precisamente en tracking persistente y grafos de dependencias para agentes, así que interoperar sería preferible a reconstruir su dominio.

---

# 129. Posible OpenSpec integration

AWC podría adoptar:

```text
openspec/changes/**
```

como artifacts externos.

AWC no los modificaría necesariamente.

Solo conocería:

```text
identity
location
status
ownership
```

OpenSpec seguiría controlando su workflow de especificaciones.

---

# 130. Providers futuros

No definir todavía:

```rust
trait ArtifactProvider
trait WorkProvider
trait SecretProvider
```

hasta tener al menos dos implementaciones reales.

Primero:

```text
concrete implementation
```

Después:

```text
extract abstraction
```

Esto reduce complejidad innecesaria mientras se aprende Rust.

---

# 131. Flujo completo: feature compleja

Usuario:

> Planea una feature de notificaciones para Foodly. Incluye frontend y backend.

Agent Skill clasifica:

```text
complex plan
```

El agente utiliza:

```text
awc_plan_create
```

AWC crea:

```text
artifacts/plans/foodly/notifications/
```

El agente analiza primero y posteriormente crea:

```text
WORK-01 Domain
WORK-02 Persistence
WORK-03 API
WORK-04 Worker
WORK-05 Frontend client
WORK-06 Notification UI
WORK-07 Integration tests
```

Dependencias:

```text
01
 ↓
02
 ↓
03
 ├──────► 04
 ▼
05
 ↓
06
 ↓
07
```

Resultado físico:

```text
notifications/
├── README.md
├── architecture.md
└── work/
    ├── BE-001-domain.md
    ├── BE-002-persistence.md
    ├── BE-003-api.md
    ├── BE-004-worker.md
    ├── FE-001-client.md
    ├── FE-002-ui.md
    └── QA-001-integration.md
```

---

# 132. Flujo completo: code review

Usuario:

> Revisa el PR 284 de FlyAdd.

Agent Skill determina:

```text
persistent CodeReview artifact
```

Llama:

```text
awc_artifact_create
```

AWC devuelve:

```text
ART-ID
artifacts/reviews/flyadd/2026-08-08-pr-284.md
```

El agente analiza el PR y escribe ahí.

AWC no interpreta los findings.

Solo conserva y gobierna el artefacto.

---

# 133. Flujo completo: agente rompe el protocolo

El agente hace:

```bash
cat > plan-final.md
```

La sesión termina.

Posteriormente:

```bash
awctl doctor
```

encuentra:

```text
UNMANAGED_ROOT_FILE
plan-final.md
```

AWC recomienda:

```text
adopt as artifact
move to inbox
ignore explicitly
```

No elimina.

El sistema continúa siendo consistente aunque el agente haya ignorado la Skill.

---

# 134. Flujo completo: archivo movido

Existe:

```text
ART-123
artifacts/reviews/foo.md
SHA256 ABC
```

El agente lo mueve manualmente.

Doctor encuentra:

```text
missing ART-123
```

y posteriormente:

```text
docs/foo.md
SHA256 ABC
```

Si es el único candidato:

```text
probable move: HIGH confidence
```

y propone `relink`.

---

# 135. Flujo completo: secreto

Usuario proporciona una SSH key.

El agente no debe convertirla en:

```text
registry/credentials.md
```

AWC puede registrar:

```text
secret reference:
flyadd-prod-ssh
provider: ssh-agent/config
alias: flyadd-prod
```

Después se utiliza:

```bash
ssh flyadd-prod
```

Sin necesidad de persistir la key dentro de AWC.

---

# 136. UX humana

Ejemplo:

```text
$ awctl status

Agent Workspace Control

Workspace
  Health        warning
  Projects      4
  Artifacts     37
  Active work   5
  Blocked       1

Hygiene
  Unmanaged     2
  Temporary     3
  Expired       1

Security
  References    8
  Findings      0

Integrations
  OpenClaw      ready
  Hermes        available

Run `awctl doctor` for details.
```

---

# 137. UX de agente

```bash
awctl context --json
```

```json
{
  "schemaVersion": 1,
  "ok": true,
  "data": {
    "health": "warning",
    "activeWork": [
      {
        "id": "WORK-019c...",
        "title": "Notification API"
      }
    ],
    "warnings": [
      {
        "code": "UNMANAGED_FILE",
        "path": "plan-final.md"
      }
    ]
  }
}
```

Conciso.

Accionable.

Parseable.

---

# 138. Filosofía de performance

AWC manejará workspaces relativamente pequeños en comparación con bases de datos tradicionales.

Por tanto se prioriza:

```text
correctness
clarity
recovery
```

sobre micro-optimizaciones.

`doctor --quick` evita operaciones caras en hot paths.

Los hashes completos se calculan únicamente cuando aportan valor.

---

# 139. Filosofía de Rust

El proyecto debe utilizar Rust para ganar:

* tipos explícitos;
* enums;
* newtypes;
* robustez de errores;
* binario distribuible;
* control del filesystem;
* bajo overhead.

No debe convertirse en un ejercicio de usar todas las características avanzadas de Rust.

Evitar inicialmente:

```text
heavy generics
deep trait hierarchies
async everywhere
macro-heavy architecture
unsafe
complex lifetimes
actor systems
```

---

# 140. Camino de aprendizaje de Rust

## Fase 1

Aprender:

```text
Cargo
modules
struct
enum
match
Result
Option
```

Implementar:

```text
init
status
```

## Fase 2

Aprender:

```text
Path
PathBuf
std::fs
borrowing
ownership
```

Implementar artifact filesystem.

## Fase 3

Aprender:

```text
Serde
generics sencillos
newtypes
```

Implementar JSON contracts.

## Fase 4

Aprender:

```text
SQLite
transactions
iterators
error conversion
```

## Fase 5

Aprender:

```text
HashMap
HashSet
graphs
SHA-256
```

Implementar reconciliation.

## Fase 6

Aprender async únicamente dentro de:

```text
awc-mcp
```

con:

```text
Tokio
rmcp
```

---

# 141. Riesgos principales

## Riesgo 1 — El agente ignora AWC

Mitigación:

```text
Skill
context
doctor
adopt
cleanup
native adapter futuro
```

La V1 se diseña para recuperación, no obediencia perfecta.

---

## Riesgo 2 — Scope creep hacia Beads

Mitigación:

WorkItem deliberadamente pequeño.

Si se requieren features avanzadas, integrar un provider externo.

---

## Riesgo 3 — Scope creep hacia OpenSpec

Mitigación:

AWC gobierna artefactos.

No prescribe cómo escribir una especificación.

---

## Riesgo 4 — Plugin lock-in

Mitigación:

```text
Rust core
CLI
MCP
portable Skill
```

Plugins nativos siempre opcionales.

---

## Riesgo 5 — Destructive cleanup

Mitigación:

```text
scan
plan
review
apply
trash
retention
purge
```

más hashes y preconditions.

---

## Riesgo 6 — SQLite/filesystem divergence

Mitigación:

```text
transactions
temporary writes
atomic moves
hashes
doctor
reconciliation
audit
```

---

## Riesgo 7 — Secret exposure

Mitigación:

```text
references only
no resolve
security scan
redacted logs
```

---

# 142. Decisiones arquitectónicas definitivas

## ADR-001

**AWC será un programa independiente, no un plugin de OpenClaw.**

Motivo:

portabilidad.

---

## ADR-002

**Rust será el lenguaje del core y CLI.**

---

## ADR-003

**SQLite almacenará metadata; filesystem almacenará contenido.**

---

## ADR-004

**El agente puede editar contenido directamente.**

AWC gobierna lifecycle, no prosa.

---

## ADR-005

**CLI será interfaz universal.**

---

## ADR-006

**MCP será interfaz preferida agent → AWC.**

---

## ADR-007

**AgentSkill será responsable del protocolo de comportamiento.**

---

## ADR-008

**Plugins nativos serán adaptadores opcionales de lifecycle/enforcement.**

---

## ADR-009

**No habrá Task e Issue separados.**

Se utilizará `WorkItem`.

---

## ADR-010

**Cleanup será scan → plan → apply.**

---

## ADR-011

**Uncertainty means preserve.**

---

## ADR-012

**AWC nunca será un vault.**

---

## ADR-013

**No se utilizará async en el core.**

---

## ADR-014

**MCP async estará aislado en `awc-mcp`.**

---

## ADR-015

**Linux es Tier 1.**

---

# 143. Definición del producto

AWC no es:

> Un mejor prompt para que un agente sea organizado.

Es:

> **Una capa de control determinista alrededor de los efectos persistentes de un agente.**

Su arquitectura final puede resumirse así:

```text
                         USER
                           │
                           ▼
                    ┌─────────────┐
                    │    Agent    │
                    │ OpenClaw /  │
                    │   Hermes    │
                    └──────┬──────┘
                           │
                  ┌────────┴────────┐
                  │                 │
                  ▼                 ▼
              AgentSkill          MCP
            when / why             │
                  │                 │
                  └────────┬────────┘
                           ▼
                    ┌─────────────┐
                    │  AWC Core   │
                    │    Rust     │
                    └──────┬──────┘
                           │
            ┌──────────────┼───────────────┐
            ▼              ▼               ▼
        Artifacts        Policies       Hygiene
            │              │               │
            └──────────────┼───────────────┘
                           ▼
                ┌───────────────────┐
                │ SQLite + FS       │
                └───────────────────┘

              Human administration
                       │
                       ▼
                     awctl


Optional host-specific layer:

OpenClaw Hooks ─┐
                ├──► AWC policies
Hermes Hooks ───┘
```

---

# 144. Resultado esperado

Después de meses de operación, no debería ser necesario revisar cientos de conversaciones para comprender el estado del agente.

Debería bastar con:

```bash
awctl status
```

y:

```bash
awctl context --json
```

para conocer:

* proyectos activos;
* trabajo pendiente;
* work items bloqueados;
* planes;
* reviews;
* investigaciones;
* temporales;
* inconsistencias;
* artefactos recientes;
* referencias de credenciales;
* problemas de higiene.

Y si el agente vuelve a crear:

```text
random-plan-final-v2.md
```

AWC no habrá fracasado.

AWC habrá fracasado únicamente si ese archivo puede permanecer indefinidamente sin que el sistema pueda detectarlo, explicarlo y ofrecer una forma segura de reconciliarlo.

Ésa es la garantía sobre la que debe construirse Agent Workspace Control.