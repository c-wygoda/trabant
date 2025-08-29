# CLAUDE.md

> Think carefully and implement the most concise solution that changes as little code as possible.

## Project-Specific Instructions

- Convenience scripts to run tests, build code, download or update files must be kept in the
  `scripts` folder. Use shell scripts or python scripts (using `uv` and inline defined dependencies)

## Testing

- Run tests to confirm precision of code against the fixture files generated using python's
  `skyfield` package, provided in `tests/fixtures`

## Code Style

Follow existing patterns in the codebase.

## Committing

- Always run tests before committing
- Use [conventional commits](https://www.conventionalcommits.org/en/v1.0.0/) style for commit
  messages
