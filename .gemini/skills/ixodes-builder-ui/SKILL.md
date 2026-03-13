---
name: ixodes-builder-ui
description: Expert in modifying the Svelte-based frontend for the Ixodes GUI builder.
---

# Ixodes Builder UI

You are an expert in the `ixodes-gui` project's Svelte 5 frontend. Your goal is to help users improve the user interface, add new configuration fields, and ensure a seamless experience for building the `ixodes` agent.

## Core Concepts

- **Svelte 5 Runes:** Uses `$state`, `$derived`, and `$effect` for state management.
- **Tauri Integration:** Uses `invoke` from `@tauri-apps/api/core` to communicate with the Rust backend.
- **Component Architecture:** Main UI is in `src/routes/+page.svelte`, with specialized sections extracted into `src/routes/components/`.
- **UI Framework:** Uses Tailwind CSS v4 and shadcn-svelte components.

## Key Files

- `ixodes-gui/src/routes/+page.svelte`: The primary build configuration interface.
- `ixodes-gui/src/routes/components/`: Sub-components for specific configuration categories (e.g., `CommunicationSection.svelte`, `ClipperSection.svelte`).
- `ixodes-gui/src/lib/components/ui/`: Base UI components (Button, Input, Switch, etc.).

## Workflow for Adding a New UI Field

1.  **Define State:**
    Add a new `$state` variable in `+page.svelte` or the relevant sub-component.
    ```typescript
    let myNewSetting = $state(false);
    ```

2.  **Add UI Component:**
    Use a shadcn-svelte component (e.g., `Switch`, `Input`) to allow the user to modify the state.
    ```svelte
    <div class="flex items-center space-x-2">
      <Switch id="my-setting" bind:checked={myNewSetting} />
      <Label for="my-setting">Enable My New Setting</Label>
    </div>
    ```

3.  **Update Build Request:**
    Ensure the new state is included in the `BuildRequest` object sent to the `build_ixodes` command.

4.  **Validation:**
    Use `$derived` for any validation logic and display feedback using `toast` (from `svelte-sonner`).

## Best Practices

- **Reactive State:** Always use Svelte 5 runes for reactivity. Avoid legacy Svelte 4 syntax (`let` without `$state`).
- **Modularity:** Keep `+page.svelte` clean by moving complex sections into separate components in the `components/` directory.
- **Styling:** Use Tailwind CSS for all styling. Follow the existing "modern/dark" aesthetic.
- **User Feedback:** Use `toast.message` or `toast.error` to inform the user about the build progress or validation failures.
