# MODULES — Module Catalog

| ID | Module | Layer | Interfaces | Depends on | Status |
|---|---|---|---|---|---|
| CORE-01 | Project Manager | Application | IProjectManager | FileSystem, Metadata | PLANNED |
| CORE-02 | File System Abstraction | Infrastructure | IFileSystem | — | PLANNED |
| CORE-03 | Project Metadata | Domain | ProjectMeta | — | PLANNED |
| CORE-04 | Migration Session | Application | ISession | Event Bus, Checkpoint | PLANNED |
| CORE-05 | Event Bus | Infrastructure | IEventBus | — | PLANNED |
| CORE-06 | Checkpoint / Rollback | Infrastructure | ICheckpoint | FileSystem | PLANNED |
| CORE-07 | Job Persistence | Infrastructure | IJobStore | Database | PLANNED |
| ENG-01 | Analyzer Engine | Application | IAnalyzer | JAR, Source, Build | PLANNED |
| ENG-02 | Compatibility Engine | Application | ICompatibility | Knowledge Base | PLANNED |
| ENG-03 | Migration Engine | Application | IMigration | Analyzer, Compatibility, Transform | PLANNED |
| ENG-04 | Build Engine | Application | IBuild | Toolchain, Sandbox | PLANNED |
| ENG-05 | Testing Engine | Application | ITestRunner | Build, Launcher | PLANNED |
| AI-01 | AI Provider Abstraction | Infrastructure | IAIProvider | — | PLANNED |
| AI-02 | Agent Orchestrator | Application | IAgent | AI-01, Context | PLANNED |
| PLG-01 | Plugin Registry | Infrastructure | IPluginRegistry | — | PLANNED |
| PLG-02 | Plugin Loader | Infrastructure | IPluginLoader | PLG-01 | PLANNED |
| PLG-03 | Plugin SDK | — | SDK docs + API | — | PLANNED |