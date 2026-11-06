<script lang="ts">
  // Monta o diálogo como o App.svelte: props lidas de `ui.moveRequest` dentro do `{#if}`.
  import MoveRequestDialog from '../MoveRequestDialog.svelte';
  import { ui } from '../../state/ui.svelte';
</script>

{#if ui.moveRequest}<MoveRequestDialog path={ui.moveRequest.path} name={ui.moveRequest.name} />{/if}
