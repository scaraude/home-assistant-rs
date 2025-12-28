<script lang="ts">
  import type { TopConsumerEntry } from './api';
  import ProcessHistoryGraph from './ProcessHistoryGraph.svelte';

  let {
    entries = [],
    type = 'cpu'
  }: {
    entries?: TopConsumerEntry[];
    type?: 'cpu' | 'ram';
  } = $props();

  let expandedProcess = $state<string | null>(null);

  // Get the latest snapshot (most recent timestamp)
  let latestEntries = $derived(getLatestSnapshot(entries));

  function getLatestSnapshot(allEntries: TopConsumerEntry[]) {
    if (allEntries.length === 0) return [];

    // Find the most recent timestamp
    const latestTimestamp = allEntries.reduce((latest, entry) => {
      const entryTime = new Date(entry.timestamp).getTime();
      return entryTime > latest ? entryTime : latest;
    }, 0);

    // Get all entries with that timestamp and sort by rank
    return allEntries
      .filter((e) => new Date(e.timestamp).getTime() === latestTimestamp)
      .sort((a, b) => a.rank - b.rank);
  }

  function getRankBadgeClass(rank: number): string {
    if (rank <= 3) return 'rank-top';
    if (rank <= 6) return 'rank-mid';
    return 'rank-low';
  }

  function getProgressBarWidth(value: number, max: number): number {
    return Math.min((value / max) * 100, 100);
  }

  function toggleExpand(processKey: string) {
    expandedProcess = expandedProcess === processKey ? null : processKey;
  }

  function getProcessKey(entry: TopConsumerEntry): string {
    return `${entry.process}-${entry.pid}`;
  }

  let maxCpu = $derived(latestEntries.length > 0 ? Math.max(...latestEntries.map((e) => e.cpu)) : 100);
  let maxRam = $derived(latestEntries.length > 0 ? Math.max(...latestEntries.map((e) => e.ram)) : 100);
</script>

<div class="top-consumers-container">
  {#if latestEntries.length === 0}
    <div class="empty-state">
      <p>No {type === 'cpu' ? 'CPU' : 'RAM'} consumer data available</p>
    </div>
  {:else}
    <div class="header">
      <h3>Top {type === 'cpu' ? 'CPU' : 'RAM'} Consumers</h3>
      <div class="last-updated">
        Last updated: {new Date(latestEntries[0].timestamp).toLocaleString()}
      </div>
    </div>

    <div class="table-wrapper">
      <table>
        <thead>
          <tr>
            <th>Rank</th>
            <th>Process</th>
            <th>PID</th>
            <th>CPU (%)</th>
            <th>RAM (MB)</th>
            <th>{type === 'cpu' ? 'CPU Usage' : 'RAM Usage'}</th>
          </tr>
        </thead>
        <tbody>
          {#each latestEntries as entry (entry.rank + entry.pid)}
            {@const processKey = getProcessKey(entry)}
            {@const isExpanded = expandedProcess === processKey}
            <tr
              class="clickable-row"
              class:expanded={isExpanded}
              onclick={() => toggleExpand(processKey)}
            >
              <td>
                <span class="rank-badge {getRankBadgeClass(entry.rank)}">
                  #{entry.rank}
                </span>
              </td>
              <td class="process-name" title={entry.process}>
                <span class="expand-icon">{isExpanded ? '▼' : '▶'}</span>
                {entry.process.length > 40 ? entry.process.substring(0, 37) + '...' : entry.process}
              </td>
              <td>{entry.pid}</td>
              <td>{entry.cpu.toFixed(1)}%</td>
              <td>{entry.ram.toFixed(1)}</td>
              <td class="progress-cell">
                <div class="progress-bar-container">
                  <div
                    class="progress-bar {type === 'cpu' ? 'progress-cpu' : 'progress-ram'}"
                    style="width: {getProgressBarWidth(
                      type === 'cpu' ? entry.cpu : entry.ram,
                      type === 'cpu' ? maxCpu : maxRam
                    )}%"
                  ></div>
                </div>
              </td>
            </tr>
            {#if isExpanded}
              <tr class="expanded-row">
                <td colspan="6" class="expanded-cell">
                  <ProcessHistoryGraph
                    processName={entry.process}
                    pid={entry.pid}
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
  .top-consumers-container {
    width: 100%;
    background: white;
    border: 1px solid #e5e7eb;
    border-radius: 8px;
    overflow: hidden;
  }

  .header {
    padding: 1rem 1.25rem;
    border-bottom: 1px solid #e5e7eb;
    background-color: #f9fafb;
  }

  .header h3 {
    margin: 0 0 0.25rem 0;
    font-size: 1.125rem;
    font-weight: 600;
    color: #111827;
  }

  .last-updated {
    font-size: 0.75rem;
    color: #6b7280;
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
    font-size: 0.8125rem;
    max-width: 300px;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    display: flex;
    align-items: center;
    gap: 0.5rem;
  }

  .expand-icon {
    display: inline-block;
    font-size: 0.625rem;
    color: #6b7280;
    flex-shrink: 0;
    transition: transform 0.2s;
  }

  .rank-badge {
    display: inline-block;
    padding: 0.25rem 0.5rem;
    border-radius: 4px;
    font-size: 0.75rem;
    font-weight: 600;
  }

  .rank-top {
    background-color: #fef3c7;
    color: #92400e;
  }

  .rank-mid {
    background-color: #dbeafe;
    color: #1e40af;
  }

  .rank-low {
    background-color: #f3f4f6;
    color: #374151;
  }

  .progress-cell {
    min-width: 150px;
  }

  .progress-bar-container {
    width: 100%;
    height: 20px;
    background-color: #f3f4f6;
    border-radius: 4px;
    overflow: hidden;
  }

  .progress-bar {
    height: 100%;
    transition: width 0.3s ease;
  }

  .progress-cpu {
    background: linear-gradient(90deg, #3b82f6, #2563eb);
  }

  .progress-ram {
    background: linear-gradient(90deg, #22c55e, #16a34a);
  }

  @media (max-width: 768px) {
    table {
      font-size: 0.75rem;
    }

    th,
    td {
      padding: 0.5rem;
    }

    .process-name {
      max-width: 150px;
    }
  }
</style>
