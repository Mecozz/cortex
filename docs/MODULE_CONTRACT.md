# Module Contract

Every module in Cortex must follow this contract. CI enforces it.

## Required files

- src-tauri/src/<module>/mod.rs � public API, no cross-module imports
- src-tauri/src/<module>/health.rs � implements the HealthCheck trait
- README.md in the module directory (optional for small modules, required for Phase 4+)

## Required impls

### HealthCheck

Every module must implement core::health::HealthCheck:
`ust
impl HealthCheck for MyModule {
    fn module_name(&self) -> &str {  my_module }
    fn health(&self) -> HealthReport { /* green/yellow/red */ }
}`

### Reset

Every module that holds state must expose a reset function:
`ust
pub fn reset(&self) -> Result<()> { /* clear module state */ }`

## Rules

- No direct imports across module boundaries � use BUS only
- Files max 400 lines, functions max 50 lines
- All public functions must have at least one test
