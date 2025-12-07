<script lang="ts">
  import TemperatureGraph from './TemperatureGraph.svelte';
  import GraphModal from './GraphModal.svelte';
  import type { SensorData } from './api';

  export let sensorData: SensorData;

  type Metric = 'temperature' | 'humidity' | 'battery' | null;
  let selectedMetric: Metric = null;
  let isModalOpen = false;

  $: latest = sensorData.latestReading;
  $: displayName = sensorData.id.slice(0, 16) + (sensorData.id.length > 16 ? '...' : '');

  function toggleMetric(metric: Metric) {
    if (selectedMetric === metric) {
      selectedMetric = null; // Toggle off if clicking the same metric
    } else {
      selectedMetric = metric;
    }
  }

  function openModal() {
    isModalOpen = true;
  }
</script>

<div class="sensor-card">
  <div class="card-header">
    <div class="sensor-icon">
      <!-- Material Design Thermostat Icon -->
      <svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="currentColor">
        <path d="M15 13V5c0-1.66-1.34-3-3-3S9 3.34 9 5v8c-1.21.91-2 2.37-2 4 0 2.76 2.24 5 5 5s5-2.24 5-5c0-1.63-.79-3.09-2-4zm-4-2V5c0-.55.45-1 1-1s1 .45 1 1v6h-2z"/>
      </svg>
    </div>
    <div class="sensor-info">
      <h3 class="sensor-name" title={sensorData.id}>{displayName}</h3>
      {#if latest}
        <div class="last-update">
          {new Date(latest.timestamp * 1000).toLocaleString()}
        </div>
      {/if}
    </div>
    {#if latest && latest.link_quality !== null}
      <div class="link-quality-badge" class:good={latest.link_quality >= 100} class:medium={latest.link_quality >= 50 && latest.link_quality < 100} class:poor={latest.link_quality < 50}>
        <svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="currentColor">
          <path d="M1 9l2 2c4.97-4.97 13.03-4.97 18 0l2-2C16.93 2.93 7.08 2.93 1 9zm8 8l3 3 3-3c-1.65-1.66-4.34-1.66-6 0zm-4-4l2 2c2.76-2.76 7.24-2.76 10 0l2-2C15.14 9.14 8.87 9.14 5 13z"/>
        </svg>
        {latest.link_quality}
      </div>
    {/if}
  </div>

  {#if latest}
    <div class="readings">
      <button
        class="reading-item"
        class:active={selectedMetric === 'temperature'}
        on:click={() => toggleMetric('temperature')}
        type="button"
      >
        <div class="reading-icon">
          <!-- Temperature Icon -->
          <svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="currentColor">
            <path d="M15 13V5c0-1.66-1.34-3-3-3S9 3.34 9 5v8c-1.21.91-2 2.37-2 4 0 2.76 2.24 5 5 5s5-2.24 5-5c0-1.63-.79-3.09-2-4z"/>
          </svg>
        </div>
        <div class="reading-value">
          <span class="value">{latest.temperature.toFixed(1)}</span>
          <span class="unit">°C</span>
        </div>
      </button>

      {#if latest.humidity !== null}
        <button
          class="reading-item"
          class:active={selectedMetric === 'humidity'}
          on:click={() => toggleMetric('humidity')}
          type="button"
        >
          <div class="reading-icon">
            <!-- Humidity Icon -->
            <svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="currentColor">
              <path d="M12 2c-5.33 4.55-8 8.48-8 11.8 0 4.98 3.8 8.2 8 8.2s8-3.22 8-8.2c0-3.32-2.67-7.25-8-11.8z"/>
            </svg>
          </div>
          <div class="reading-value">
            <span class="value">{latest.humidity.toFixed(1)}</span>
            <span class="unit">%</span>
          </div>
        </button>
      {/if}

      {#if latest.battery !== null}
        <button
          class="reading-item"
          class:active={selectedMetric === 'battery'}
          on:click={() => toggleMetric('battery')}
          type="button"
        >
          <div class="reading-icon">
            <!-- Battery Icon -->
            <svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="currentColor">
              <path d="M15.67 4H14V2h-4v2H8.33C7.6 4 7 4.6 7 5.33v15.33C7 21.4 7.6 22 8.33 22h7.33c.74 0 1.34-.6 1.34-1.33V5.33C17 4.6 16.4 4 15.67 4z"/>
            </svg>
          </div>
          <div class="reading-value">
            <span class="value">{latest.battery}</span>
            <span class="unit">%</span>
          </div>
        </button>
      {/if}
    </div>

    <div class="graph-section" on:click={openModal} role="button" tabindex="0" on:keydown={(e) => e.key === 'Enter' && openModal()}>
      <TemperatureGraph readings={sensorData.history} selectedMetric={selectedMetric} />
      <div class="zoom-hint">
        <svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="currentColor">
          <path d="M15.5 14h-.79l-.28-.27C15.41 12.59 16 11.11 16 9.5 16 5.91 13.09 3 9.5 3S3 5.91 3 9.5 5.91 16 9.5 16c1.61 0 3.09-.59 4.23-1.57l.27.28v.79l5 4.99L20.49 19l-4.99-5zm-6 0C7.01 14 5 11.99 5 9.5S7.01 5 9.5 5 14 7.01 14 9.5 11.99 14 9.5 14z"/>
          <path d="M12 10h-2v2H9v-2H7V9h2V7h1v2h2v1z"/>
        </svg>
        Click to zoom
      </div>
    </div>
  {:else}
    <div class="no-data">
      <p>No data available</p>
    </div>
  {/if}
</div>

<GraphModal
  bind:isOpen={isModalOpen}
  readings={sensorData.history}
  {selectedMetric}
  sensorName={sensorData.id}
/>

<style>
  .sensor-card {
    background: white;
    border: 1px solid #e5e7eb;
    border-radius: 8px;
    padding: 1.25rem;
    box-shadow: 0 1px 3px rgba(0, 0, 0, 0.1);
    transition: box-shadow 0.2s ease;
  }

  .sensor-card:hover {
    box-shadow: 0 4px 6px rgba(0, 0, 0, 0.1);
  }

  .card-header {
    display: flex;
    align-items: center;
    gap: 0.75rem;
    margin-bottom: 1rem;
  }

  .sensor-icon {
    width: 40px;
    height: 40px;
    background: linear-gradient(135deg, #3b82f6 0%, #2563eb 100%);
    border-radius: 8px;
    display: flex;
    align-items: center;
    justify-content: center;
    color: white;
  }

  .sensor-icon svg {
    width: 24px;
    height: 24px;
  }

  .sensor-info {
    flex: 1;
    min-width: 0;
  }

  .sensor-name {
    margin: 0;
    font-size: 1rem;
    font-weight: 600;
    color: #111827;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .last-update {
    font-size: 0.75rem;
    color: #6b7280;
    margin-top: 0.125rem;
  }

  .link-quality-badge {
    display: flex;
    align-items: center;
    gap: 0.25rem;
    padding: 0.25rem 0.5rem;
    border-radius: 12px;
    font-size: 0.75rem;
    font-weight: 600;
    background: rgba(107, 114, 128, 0.1);
    color: #6b7280;
    border: 1px solid rgba(107, 114, 128, 0.2);
    margin-left: auto;
  }

  .link-quality-badge svg {
    width: 14px;
    height: 14px;
  }

  .link-quality-badge.good {
    background: rgba(34, 197, 94, 0.1);
    color: #16a34a;
    border-color: rgba(34, 197, 94, 0.3);
  }

  .link-quality-badge.medium {
    background: rgba(251, 191, 36, 0.1);
    color: #d97706;
    border-color: rgba(251, 191, 36, 0.3);
  }

  .link-quality-badge.poor {
    background: rgba(239, 68, 68, 0.1);
    color: #dc2626;
    border-color: rgba(239, 68, 68, 0.3);
  }

  .readings {
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(100px, 1fr));
    gap: 0.75rem;
    margin-bottom: 1.25rem;
  }

  .reading-item {
    display: flex;
    align-items: center;
    gap: 0.5rem;
    padding: 0.5rem;
    background: #f9fafb;
    border-radius: 6px;
    border: 2px solid transparent;
    cursor: pointer;
    transition: all 0.2s ease;
    width: 100%;
    text-align: left;
  }

  .reading-item:hover {
    background: #f3f4f6;
    border-color: #e5e7eb;
    transform: translateY(-1px);
  }

  .reading-item.active {
    background: #6aabff;
    border-color: #3b82f6;
  }

  .reading-item.active .reading-icon {
    color: #3b82f6;
  }

  .reading-item:active {
    transform: translateY(0);
  }

  .reading-icon {
    width: 24px;
    height: 24px;
    color: #6b7280;
  }

  .reading-icon svg {
    width: 100%;
    height: 100%;
  }

  .reading-value {
    display: flex;
    align-items: baseline;
    gap: 0.25rem;
  }

  .value {
    font-size: 1.125rem;
    font-weight: 600;
    color: #111827;
  }

  .unit {
    font-size: 0.875rem;
    color: #6b7280;
  }

  .graph-section {
    margin-top: 1rem;
    position: relative;
    cursor: pointer;
    border-radius: 8px;
    transition: all 0.2s ease;
    padding: 0.5rem;
    margin: 0.5rem -0.5rem 0;
  }

  .graph-section:hover {
    background: #f9fafb;
    transform: scale(1.01);
  }

  .graph-section:active {
    transform: scale(0.99);
  }

  .zoom-hint {
    position: absolute;
    top: 1rem;
    right: 1rem;
    display: flex;
    align-items: center;
    gap: 0.25rem;
    background: rgba(255, 255, 255, 0.95);
    backdrop-filter: blur(4px);
    padding: 0.375rem 0.625rem;
    border-radius: 6px;
    font-size: 0.75rem;
    color: #6b7280;
    border: 1px solid #e5e7eb;
    opacity: 0;
    transition: opacity 0.2s ease;
    pointer-events: none;
  }

  .graph-section:hover .zoom-hint {
    opacity: 1;
  }

  .zoom-hint svg {
    width: 14px;
    height: 14px;
  }

  .no-data {
    padding: 2rem;
    text-align: center;
    color: #6b7280;
  }

  .no-data p {
    margin: 0;
  }
</style>
