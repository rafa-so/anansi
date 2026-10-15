<script lang="ts">
  // Preferências globais (Seção 10): padrões de TLS e timeout.
  import { onMount } from 'svelte';
  import { prefsApi } from '../api/http';
  import { toasts } from '../state/dialogs.svelte';
  import { ui } from '../state/ui.svelte';
  import { errorMessage } from '../types';
  import Modal from './Modal.svelte';

  let verifyTls = $state(true);
  let timeoutSeconds = $state(30);
  let loaded = $state(false);

  onMount(() => {
    prefsApi
      .get()
      .then((p) => {
        verifyTls = p.defaultVerifyTls;
        timeoutSeconds = p.defaultTimeoutMs / 1000;
        loaded = true;
      })
      .catch((e) => toasts.error(errorMessage(e)));
  });

  async function save() {
    try {
      await prefsApi.set({
        defaultVerifyTls: verifyTls,
        defaultTimeoutMs: Math.round(timeoutSeconds * 1000),
      });
      ui.prefsOpen = false;
    } catch (e) {
      toasts.error(errorMessage(e));
    }
  }
</script>

<Modal title="Preferências" onclose={() => (ui.prefsOpen = false)}>
  <div class="form">
    <label class="check">
      <input type="checkbox" bind:checked={verifyTls} />
      Verificar certificados TLS por padrão
    </label>
    <p class="muted">
      Desligue só se precisar chamar servidores com certificado autoassinado. Cada request pode mudar isso em
      Configurações.
    </p>
    <label>
      <span>Timeout padrão (segundos)</span>
      <input type="number" min="1" bind:value={timeoutSeconds} />
    </label>
  </div>
  {#snippet footer()}
    <button onclick={() => (ui.prefsOpen = false)}>Cancelar</button>
    <button class="primary" onclick={save} disabled={!loaded || !(timeoutSeconds > 0)}>Salvar</button>
  {/snippet}
</Modal>

<style>
  .form {
    display: grid;
    gap: 8px;
  }
  .check {
    display: flex;
    gap: 8px;
    align-items: center;
  }
  label {
    display: grid;
    gap: 4px;
  }
  label span {
    color: var(--text-muted);
  }
  p {
    margin: 0 0 8px;
  }
</style>
