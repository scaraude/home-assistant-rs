<script lang="ts">
  import type { ProcessMonitorEntry } from './api';
  import ProcessHistoryGraph from './ProcessHistoryGraph.svelte';

  export let entries: ProcessMonitorEntry[] = [];
  export let timeRange: '1h' | '6h' | '24h' | 'all' = '24h';

  let expandedProcess: string | null = null;

  // Group entries by process and get the latest entry for each
  $: latestProcesses = getLatestProcesses(entries);

  function getLatestProcesses(allEntries: ProcessMonitorEntry[]) {
    const processMap = new Map<string, ProcessMonitorEntry>();

    // Group by process name and keep the latest entry
    for (const entry of allEntries) {
      const existing = processMap.get(entry.process);
      if (!existing || new Date(entry.timestamp) > new Date(existing.timestamp)) {
        processMap.set(entry.process, entry);
      }
    }

    // Convert to array and sort by process name
    return Array.from(processMap.values()).sort((a, b) =>
      a.process.localeCompare(b.process)
    );
  }

  function getStatusBadgeClass(status: string): string {
    return status.toLowerCase().includes('running') ? 'status-running' : 'status-stopped';
  }

  function toggleExpand(processKey: string) {
    expandedProcess = expandedProcess === processKey ? null : processKey;
  }

  function getProcessKey(entry: ProcessMonitorEntry): string {
    return `${entry.process}-${entry.pid}`;
  }
</script>

<div class="process-table-container">
  {#if latestProcesses.length === 0}
    <div class="empty-state">
      <p>No process data available</p>
    </div>
  {:else}
    <div class="table-wrapper">
      <table>
        <thead>
          <tr>
            <th>Process</th>
            <th>PID</th>
            <th>CPU (%)</th>
            <th>RAM (MB)</th>
            <th>Status</th>
            <th>Last Updated</th>
          </tr>
        </thead>
        <tbody>
          {#each latestProcesses as entry (entry.process + entry.pid)}
            {@const processKey = getProcessKey(entry)}
            {@const isExpanded = expandedProcess === processKey}
            <tr
              class="clickable-row"
              class:expanded={isExpanded}
              on:click={() => toggleExpand(processKey)}
            >
              <td class="process-name">
                <span class="expand-icon">{isExpanded ? '▼' : '▶'}</span>
                {entry.process}
              </td>
              <td>{entry.pid}</td>
              <td>{entry.cpu.toFixed(1)}%</td>
              <td>{entry.ram.toFixed(1)} MB</td>
              <td>
                <span class="status-badge {getStatusBadgeClass(entry.status)}">
                  {entry.status}
                </span>
              </td>
              <td class="timestamp">{new Date(entry.timestamp).toLocaleString()}</td>
            </tr>
            {#if isExpanded}
              <tr class="expanded-row">
                <td colspan="6" class="expanded-cell">
                  <ProcessHistoryGraph
                    processName={entry.process}
                    pid={entry.pid}
                    timeRange={timeRange}
                  />
                </td>
              </tr>
            {/if}
          {/each}
        </tbody>
      </table>
    </div>
  {/if}
</div>

<style>
  .process-table-container {
    width: 100%;
    background: white;
    border: 1px solid #e5e7eb;
    border-radius: 8px;
    overflow: hidden;
  }

  .empty-state {
    padding: 3rem;
    text-align: center;
    color: #6b7280;
  }

  .table-wrapper {
    overflow-x: auto;
  }

  table {
    width: 100%;
    border-collapse: collapse;
    font-size: 0.875rem;
  }

  thead {
    background-color: #f9fafb;
  }

  th {
    padding: 0.75rem 1rem;
    text-align: left;
    font-weight: 600;
    color: #374151;
    border-bottom: 2px solid #e5e7eb;
    white-space: nowrap;
  }

  tbody tr.clickable-row {
    border-bottom: 1px solid #e5e7eb;
    transition: background-color 0.15s;
    cursor: pointer;
  }

  tbody tr.clickable-row:hover {
    background-color: #f9fafb;
  }

  tbody tr.clickable-row.expanded {
    background-color: #eff6ff;
  }

  tbody tr.expanded-row {
    border-bottom: 1px solid #e5e7eb;
  }

  tbody tr:last-child {
    border-bottom: none;
  }

  td {
    padding: 0.75rem 1rem;
    color: #111827;
  }

  .expanded-cell {
    padding: 0 !important;
  }

  .process-name {
    font-family: 'Courier New', monospace;
    font-weight: 500;
    display: flex;
    align-items: center;
    gap: 0.5rem;
  }

  .expand-icon {
    display: inline-block;
    font-size: 0.625rem;
    color: #6b7280;
    transition: transform 0.2s;
  }

  .timestamp {
    color: #6b7280;
    font-size: 0.75rem;
  }

  .status-badge {
    display: inline-block;
    padding: 0.25rem 0.625rem;
    border-radius: 9999px;
    font-size: 0.75rem;
    font-weight: 500;
  }

  .status-running {
    background-color: #d1fae5;
    color: #065f46;
  }

  .status-stopped {
    background-color: #fee2e2;
    color: #991b1b;
  }

  @media (max-width: 768px) {
    table {
      font-size: 0.75rem;
    }

    th,
    td {
      padding: 0.5rem;
    }
  }
</style>
