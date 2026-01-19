<script lang="ts">
  import type { Snippet } from 'svelte';

  let {
    loading = false,
    error = null,
    empty = false,
    loadingText = 'Loading...',
    emptyTitle = 'Nothing found',
    onRetry,
    emptyState,
    children
  }: {
    loading?: boolean;
    error?: string | null;
    empty?: boolean;
    loadingText?: string;
    emptyTitle?: string;
    onRetry?: () => void;
    emptyState?: Snippet;
    children: Snippet;
  } = $props();
</script>

{#if loading}
  <div class="state-container loading">
    <div class="spinner"></div>
    <p>{loadingText}</p>
  </div>
{:else if error}
  <div class="state-container error">
    <div class="icon">
      <svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="currentColor">
        <path d="M12 2C6.48 2 2 6.48 2 12s4.48 10 10 10 10-4.48 10-10S17.52 2 12 2zm1 15h-2v-2h2v2zm0-4h-2V7h2v6z"/>
      </svg>
    </div>
    <p>Error: {error}</p>
    {#if onRetry}
      <button onclick={onRetry}>Retry</button>
    {/if}
  </div>
{:else if empty}
  <div class="state-container empty">
    <div class="icon empty-icon">
      <svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="currentColor">
        <path d="M20 6h-8l-2-2H4c-1.1 0-2 .9-2 2v12c0 1.1.9 2 2 2h16c1.1 0 2-.9 2-2V8c0-1.1-.9-2-2-2zm0 12H4V6h5.17l2 2H20v10zm-8-1c1.66 0 3-1.34 3-3s-1.34-3-3-3-3 1.34-3 3 1.34 3 3 3z"/>
      </svg>
    </div>
    <h3>{emptyTitle}</h3>
    {#if emptyState}
      {@render emptyState()}
    {/if}
  </div>
{:else}
  {@render children()}
{/if}

<style>
  .state-container {
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    padding: 4rem 2rem;
    text-align: center;
  }

  .loading {
    color: #6b7280;
  }

  .spinner {
    width: 40px;
    height: 40px;
    border: 3px solid #e5e7eb;
    border-top-color: #3b82f6;
    border-radius: 50%;
    animation: spin 1s linear infinite;
    margin-bottom: 1rem;
  }

  @keyframes spin {
    to { transform: rotate(360deg); }
  }

  .loading p, .error p {
    margin: 0;
    font-size: 0.9375rem;
  }

  .error {
    color: #dc2626;
  }

  .icon {
    width: 60px;
    height: 60px;
    margin-bottom: 1rem;
  }

  .icon svg {
    width: 100%;
    height: 100%;
  }

  .error button {
    margin-top: 1rem;
    padding: 0.5rem 1rem;
    background: #3b82f6;
    color: white;
    border: none;
    border-radius: 6px;
    cursor: pointer;
    font-size: 0.875rem;
    font-weight: 500;
    transition: background 0.2s;
  }

  .error button:hover {
    background: #2563eb;
  }

  .empty {
    color: #6b7280;
  }

  .empty-icon {
    width: 80px;
    height: 80px;
    color: #d1d5db;
  }

  .empty h3 {
    margin: 0 0 0.5rem 0;
    font-size: 1.125rem;
    font-weight: 600;
    color: #111827;
  }

  .empty :global(p) {
    margin: 0;
    font-size: 0.9375rem;
    max-width: 400px;
  }
</style>
