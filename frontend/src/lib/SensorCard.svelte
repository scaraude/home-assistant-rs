<script lang="ts">
  import TemperatureGraph from './TemperatureGraph.svelte';
  import type { SensorData } from './api';

  export let sensorData: SensorData;

  $: latest = sensorData.latestReading;
  $: displayName = sensorData.id.slice(0, 16) + (sensorData.id.length > 16 ? '...' : '');
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
  </div>

  {#if latest}
    <div class="readings">
      <div class="reading-item">
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
      </div>

      {#if latest.humidity !== null}
        <div class="reading-item">
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
        </div>
      {/if}

      {#if latest.battery !== null}
        <div class="reading-item">
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
        </div>
      {/if}
    </div>

    <div class="graph-section">
      <TemperatureGraph readings={sensorData.history} />
    </div>
  {:else}
    <div class="no-data">
      <p>No data available</p>
    </div>
  {/if}
</div>

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
