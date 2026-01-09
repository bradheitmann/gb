# G3 Flock Mode Guide

**Last updated**: January 2025  
**Source of truth**: `crates/g3-ensembles/src/flock.rs`

## Purpose

Flock mode enables parallel multi-agent development by spawning multiple G3 agent instances that work on different parts of a project simultaneously. This is useful for large projects with modular architectures where independent components can be developed in parallel.

## Overview

In Flock mode:
- Multiple agent instances run concurrently
- Each agent works on a specific module or component
- Agents operate independently but share the same codebase
- Progress is tracked and coordinated centrally

```
┌─────────────────────────────────────────────────────────┐
│                    Flock Coordinator                     │
│                                                         │
│  ┌─────────┐  ┌─────────┐  ┌─────────┐  ┌─────────┐   │
│  │ Agent 1 │  │ Agent 2 │  │ Agent 3 │  │ Agent N │   │
│  │ Module A│  │ Module B│  │ Module C│  │ Module N│   │
│  └─────────┘  └─────────┘  └─────────┘  └─────────┘   │
│       │            │            │            │         │
│       ▼            ▼            ▼            ▼         │
│  ┌─────────────────────────────────────────────────┐   │
│  │              Shared Codebase                     │   │
│  └─────────────────────────────────────────────────┘   │
└─────────────────────────────────────────────────────────┘
```

## When to Use Flock Mode

**Good candidates**:
- Microservices architectures
- Projects with independent modules
- Large refactoring across multiple files
- Parallel feature development
- Test suite expansion

**Not recommended for**:
- Tightly coupled code
- Sequential dependencies
- Small projects
- Single-file changes

## Configuration

Flock mode is configured through a YAML manifest file:

```yaml
# flock.yaml
name: "my-project-flock"
description: "Parallel development of project modules"

# Global settings
settings:
  max_agents: 4
  timeout_minutes: 60
  provider: "anthropic.default"

# Agent definitions
agents:
  - name: "api-agent"
    description: "Develops the REST API layer"
    working_dir: "src/api"
    requirements: |
      Implement REST endpoints for user management:
      - GET /users
      - POST /users
      - GET /users/{id}
      - PUT /users/{id}
      - DELETE /users/{id}

  - name: "db-agent"
    description: "Develops the database layer"
    working_dir: "src/db"
    requirements: |
      Implement database models and queries:
      - User model with CRUD operations
      - Connection pooling
      - Migration support

  - name: "test-agent"
    description: "Writes integration tests"
    working_dir: "tests"
    requirements: |
      Write integration tests for:
      - API endpoints
      - Database operations
      - Error handling
```

## Usage

### Starting a Flock

```bash
# Start flock with manifest
g3 --flock flock.yaml

# Start with specific agents only
g3 --flock flock.yaml --agents api-agent,db-agent

# Start with custom timeout
g3 --flock flock.yaml --timeout 120
```

### Monitoring Progress

Flock mode provides real-time status updates:

```
🐦 Flock Status: my-project-flock
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

  api-agent     [████████░░] 80%  Implementing DELETE endpoint
  db-agent      [██████████] 100% ✅ Complete
  test-agent    [██████░░░░] 60%  Writing error handling tests

Elapsed: 15m 32s | Tokens: 45,230 | Errors: 0
```

### Stopping a Flock

```bash
# Graceful stop (wait for current tasks)
Ctrl+C

# Force stop all agents
Ctrl+C Ctrl+C
```

## Agent Communication

Agents in a flock operate independently but can:

1. **Read shared files**: All agents can read the entire codebase
2. **Write to their area**: Each agent writes to its designated working directory
3. **Signal completion**: Agents report when their tasks are done
4. **Report errors**: Failures are logged and can trigger coordinator action

### Conflict Prevention

To prevent conflicts:
- Assign non-overlapping working directories
- Use clear module boundaries
- Define explicit interfaces between modules
- Run integration after all agents complete

## Status Tracking

Flock status is tracked in `.g3/flock/`:

```
.g3/flock/
├── status.json           # Overall flock status
├── api-agent/
│   ├── session.json      # Agent session log
│   └── todo.g3.md        # Agent's TODO list
├── db-agent/
│   ├── session.json
│   └── todo.g3.md
└── test-agent/
    ├── session.json
    └── todo.g3.md
```

### Status File Format

```json
{
  "flock_name": "my-project-flock",
  "started_at": "2025-01-03T10:00:00Z",
  "status": "running",
  "agents": [
    {
      "name": "api-agent",
      "status": "running",
      "progress": 80,
      "current_task": "Implementing DELETE endpoint",
      "tokens_used": 15000,
      "errors": 0
    }
  ]
}
```

## Best Practices

### 1. Define Clear Boundaries

```yaml
# Good: Clear module separation
agents:
  - name: "frontend"
    working_dir: "src/frontend"
  - name: "backend"
    working_dir: "src/backend"

# Bad: Overlapping directories
agents:
  - name: "agent1"
    working_dir: "src"
  - name: "agent2"
    working_dir: "src/utils"  # Overlaps with agent1!
```

### 2. Specify Interfaces First

Define shared interfaces before parallel development:

```yaml
agents:
  - name: "interface-agent"
    priority: 1  # Runs first
    requirements: |
      Define shared interfaces in src/interfaces/:
      - UserService trait
      - DatabaseConnection trait
      - Error types

  - name: "impl-agent"
    priority: 2  # Runs after interfaces
    depends_on: ["interface-agent"]
    requirements: |
      Implement UserService trait...
```

### 3. Use Appropriate Granularity

- **Too few agents**: Doesn't leverage parallelism
- **Too many agents**: Coordination overhead, potential conflicts
- **Sweet spot**: 2-6 agents for most projects

### 4. Include a Test Agent

Always include an agent for testing:

```yaml
agents:
  - name: "test-agent"
    working_dir: "tests"
    requirements: |
      Write tests for all new functionality.
      Run tests after other agents complete.
```

### 5. Plan for Integration

After flock completion:

```bash
# Run all tests
cargo test

# Check for conflicts
git status

# Review changes
git diff
```

## Error Handling

### Agent Failures

If an agent fails:
1. Error is logged to agent's session
2. Coordinator is notified
3. Other agents continue (by default)
4. Failed agent can be restarted

### Restart Failed Agent

```bash
# Restart specific agent
g3 --flock flock.yaml --restart api-agent

# Restart all failed agents
g3 --flock flock.yaml --restart-failed
```

### Conflict Resolution

If agents modify the same file:
1. Last write wins (by default)
2. Conflicts are logged
3. Manual resolution may be needed

## Resource Management

### Token Usage

Each agent has its own token budget:

```yaml
settings:
  max_tokens_per_agent: 100000
  total_token_budget: 500000
```

### Concurrency

Limit concurrent agents based on:
- API rate limits
- System resources
- Provider capacity

```yaml
settings:
  max_concurrent_agents: 3  # Run at most 3 at once
```

## Example: Microservices Project

```yaml
name: "microservices-flock"

settings:
  max_agents: 5
  provider: "anthropic.default"

agents:
  - name: "user-service"
    working_dir: "services/user"
    requirements: |
      Implement user service:
      - User registration
      - Authentication
      - Profile management

  - name: "order-service"
    working_dir: "services/order"
    requirements: |
      Implement order service:
      - Order creation
      - Order status tracking
      - Payment integration

  - name: "inventory-service"
    working_dir: "services/inventory"
    requirements: |
      Implement inventory service:
      - Stock management
      - Availability checking
      - Reorder alerts

  - name: "gateway"
    working_dir: "services/gateway"
    requirements: |
      Implement API gateway:
      - Request routing
      - Authentication middleware
      - Rate limiting

  - name: "integration-tests"
    working_dir: "tests/integration"
    depends_on: ["user-service", "order-service", "inventory-service", "gateway"]
    requirements: |
      Write integration tests for:
      - End-to-end order flow
      - Service communication
      - Error scenarios
```

## Limitations

- **No real-time coordination**: Agents don't communicate during execution
- **File conflicts**: Possible if boundaries aren't clear
- **Resource intensive**: Multiple LLM calls in parallel
- **Debugging complexity**: Multiple logs to review

## Troubleshooting

### Agents Not Starting

1. Check manifest syntax (YAML)
2. Verify working directories exist
3. Check provider configuration
4. Review logs in `.g3/flock/`

### Slow Progress

1. Reduce number of concurrent agents
2. Check for rate limiting
3. Simplify requirements
4. Use faster provider

### Inconsistent Results

1. Define clearer interfaces
2. Add more specific requirements
3. Use lower temperature
4. Add validation steps
