# Contributing Guidelines

Thank you for your interest in contributing to the Rich Text Editor! This document provides guidelines and instructions for contributing.

## Table of Contents

- [Development Setup](#development-setup)
- [Code Style Guidelines](#code-style-guidelines)
- [Project Structure](#project-structure)
- [Making Changes](#making-changes)
- [Testing](#testing)
- [Pull Request Process](#pull-request-process)

---

## Development Setup

### Prerequisites

- **Node.js**: Version 18 or higher
- **npm**: Version 9 or higher
- **Git**: For version control
- **IDE**: VS Code recommended (with Svelte extension)

### Initial Setup

```bash
# Clone the repository
git clone <repository-url>
cd rich-text-editor-paginated

# Install dependencies
npm install

# Start development server
npm run dev
```

The development server runs at `http://localhost:5173` with hot module replacement.

### Recommended VS Code Extensions

- **Svelte for VS Code** (`svelte.svelte-vscode`)
- **TypeScript and JavaScript Language Features** (built-in)
- **ESLint** (`dbaeumer.vscode-eslint`)
- **Prettier** (`esbenp.prettier-vscode`)

---

## Code Style Guidelines

### TypeScript

- **Strict mode**: All code must pass TypeScript strict mode
- **Explicit types**: Use explicit type annotations for function parameters and return types
- **No `any`**: Avoid `any` type; use `unknown` if type is truly unknown
- **Interfaces over types**: Prefer `interface` for object shapes, `type` for unions/primitives

```typescript
// Good
interface UserConfig {
  name: string;
  age: number;
}

function processConfig(config: UserConfig): void {
  // ...
}

// Avoid
function processConfig(config: any) {
  // ...
}
```

### Svelte 5 Patterns

Use Svelte 5 runes consistently:

```svelte
<script lang="ts">
  // Props with $props()
  interface Props {
    title: string;
    onAction: () => void;
  }
  let { title, onAction }: Props = $props();

  // Local state with $state()
  let count = $state(0);
  let items = $state<string[]>([]);

  // Derived values with $derived
  let doubled = $derived(count * 2);
  let itemCount = $derived(items.length);

  // Effects with $effect (when needed)
  $effect(() => {
    console.log('Count changed:', count);
  });
</script>
```

### Naming Conventions

| Type | Convention | Example |
|------|------------|---------|
| Files | kebab-case | `layout-engine.ts` |
| Components | PascalCase | `ImagePopup.svelte` |
| Functions | camelCase | `handleClick()` |
| Constants | SCREAMING_SNAKE_CASE | `DEFAULT_MARGINS` |
| Types/Interfaces | PascalCase | `ParagraphMeta` |
| CSS classes | kebab-case | `.toolbar-btn` |

### JSDoc Documentation

All exported functions, types, and interfaces must have JSDoc:

```typescript
/**
 * Wraps a paragraph into multiple display lines.
 *
 * This function implements a greedy line-breaking algorithm that
 * prefers word boundaries when wrapping text.
 *
 * @param paraIndex - Index of the paragraph in the document
 * @param text - The paragraph text content
 * @param meta - Paragraph formatting metadata
 * @param measureFn - Function to measure text width
 * @returns Array of display lines
 *
 * @example
 * ```typescript
 * const lines = wrapParagraph(0, 'Hello world', meta, measureFn, config);
 * ```
 */
export function wrapParagraph(
  paraIndex: number,
  text: string,
  meta: ParagraphMeta,
  measureFn: (text: string) => number
): DisplayLine[] {
  // ...
}
```

### CSS/Styling

- Use **scoped styles** in Svelte components
- Use **CSS custom properties** for theming
- Follow **BEM-like naming** for class names
- Prefer **flexbox/grid** over floats

```svelte
<style>
  .toolbar {
    display: flex;
    gap: 8px;
  }

  .toolbar-btn {
    padding: 4px 8px;
    border: none;
    border-radius: 4px;
  }

  .toolbar-btn:hover {
    background: var(--hover-color, #f0f0f0);
  }

  .toolbar-btn.active {
    background: var(--active-color, #e0e0ff);
  }
</style>
```

---

## Project Structure

### Directory Layout

```
src/
├── App.svelte           # Root component
├── main.ts              # Entry point
└── lib/
    ├── Editor.svelte    # Main editor component
    ├── Toolbar.svelte   # Formatting toolbar
    ├── Sidebar.svelte   # Document outline
    ├── stores.ts        # Svelte stores
    ├── types.ts         # Page types
    ├── components/      # Reusable components
    │   └── *.svelte
    └── editor/          # Editor core modules
        ├── index.ts     # Module exports
        ├── types.ts     # Type definitions
        └── *.ts         # Feature modules
```

### Module Organization

Each module in `src/lib/editor/` should:

1. **Export from index.ts**: All public exports re-exported from `index.ts`
2. **Single responsibility**: Focus on one aspect of functionality
3. **Minimal dependencies**: Depend only on types and utilities
4. **Comprehensive tests**: Unit tests for all exported functions

### Import Order

```typescript
// 1. Svelte imports
import { onMount } from 'svelte';

// 2. Store imports
import { pageConfig, zoomLevel } from './stores';

// 3. Type imports
import type { ParagraphMeta, DisplayLine } from './editor/types';

// 4. Module imports
import { wrapParagraph } from './editor/text-measurement';

// 5. Component imports
import Toolbar from './Toolbar.svelte';
```

---

## Making Changes

### Branch Naming

```
feature/description-of-feature
bugfix/issue-number-description
refactor/what-is-being-refactored
docs/what-documentation
```

Examples:
- `feature/add-table-support`
- `bugfix/123-cursor-position-fix`
- `refactor/extract-layout-engine`
- `docs/api-reference`

### Commit Messages

Follow conventional commits:

```
type(scope): description

[optional body]

[optional footer]
```

Types:
- `feat`: New feature
- `fix`: Bug fix
- `refactor`: Code refactoring
- `docs`: Documentation
- `style`: Formatting changes
- `test`: Adding tests
- `chore`: Maintenance

Examples:
```
feat(editor): add support for tables

fix(images): correct crop handle positioning

refactor(layout): extract display line computation to module

docs(api): add keyboard shortcuts reference
```

### Code Changes Checklist

Before submitting changes:

- [ ] TypeScript compiles without errors (`npm run build`)
- [ ] No console errors or warnings in browser
- [ ] New functions have JSDoc documentation
- [ ] Code follows style guidelines
- [ ] Changes are tested manually
- [ ] Commit messages follow conventions

---

## Testing

### Manual Testing

For now, testing is done manually. When testing changes:

1. **Text editing**: Type, delete, navigate with keyboard
2. **Formatting**: Apply all format types (bold, italic, headings, etc.)
3. **Selection**: Select text, delete selection, copy/paste
4. **Images**: Insert, resize, crop, drag, change wrap style
5. **Page layout**: Change format, margins, orientation, columns
6. **Zoom**: Test at various zoom levels
7. **Navigation**: Use sidebar to navigate to headings

### Testing Checklist

```markdown
- [ ] Basic text input works
- [ ] Cursor navigation with arrow keys
- [ ] Selection with mouse drag
- [ ] Selection with Shift+arrows
- [ ] Copy/paste (Ctrl+C, Ctrl+V)
- [ ] Undo/redo (Ctrl+Z, Ctrl+Y)
- [ ] Bold, italic, underline formatting
- [ ] Heading styles (H1-H4)
- [ ] Bullet and numbered lists
- [ ] Text alignment
- [ ] Image insertion
- [ ] Image resize and crop
- [ ] Page format changes
- [ ] Multi-column layout
- [ ] Zoom in/out
- [ ] Sidebar navigation
```

### Future: Automated Testing

We plan to add:
- Unit tests with Vitest for utility functions
- Component tests for Svelte components
- End-to-end tests with Playwright

---

## Pull Request Process

### Before Submitting

1. **Sync with main**: Rebase or merge latest main branch
2. **Run build**: Ensure `npm run build` succeeds
3. **Manual test**: Verify your changes work as expected
4. **Update docs**: Update documentation if needed

### PR Description Template

```markdown
## Summary
Brief description of changes.

## Changes
- Added feature X
- Fixed bug Y
- Refactored Z

## Testing
How to test these changes:
1. Step one
2. Step two

## Screenshots (if applicable)
Add screenshots for UI changes.

## Checklist
- [ ] Build passes
- [ ] Manually tested
- [ ] Documentation updated
- [ ] No breaking changes (or documented)
```

### Review Process

1. **Create PR**: Submit pull request with description
2. **Automated checks**: Wait for build to pass
3. **Code review**: Address reviewer feedback
4. **Approval**: Get approval from maintainer
5. **Merge**: Squash and merge into main

### After Merge

- Delete your feature branch
- Pull latest main to local
- Verify changes in production build

---

## Getting Help

- **Issues**: Open an issue for bugs or feature requests
- **Discussions**: Use GitHub discussions for questions
- **Documentation**: Check docs folder for architecture and API reference

---

## License

By contributing, you agree that your contributions will be licensed under the same license as the project (MIT).
