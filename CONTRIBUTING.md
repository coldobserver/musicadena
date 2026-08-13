# Contributing

Thanks for your interest in Musicadena! Contributions via pull requests and bug reports are welcome.

## Getting Started

1. **Fork** the repository and clone your fork.
2. Install the [prerequisites](README.md#prerequisites) (Rust, Node.js, OS-specific Tauri deps).
3. Run the dev environment:

   ```bash
   npm install
   npm run tauri dev
   ```

## Making Changes

- Create a branch off `main`: `git checkout -b your-feature`
- Keep changes focused and make sure the app still builds:
  - Frontend: `npm run build`
  - Backend: `cargo check` (in `src-tauri/`)
- Follow the existing code style (TypeScript + React on the frontend, idiomatic Rust on the backend).

## Submitting a Pull Request

1. Push your branch to your fork and open a PR against `main`.
2. Describe what the change does and why.
3. Reference any related issue.

## Reporting Issues

- Search [issues](https://github.com/coldobserver/musicadena/issues) first to avoid duplicates.
- Include steps to reproduce, your OS, and the app version.
