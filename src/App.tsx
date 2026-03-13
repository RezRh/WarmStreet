import { createSignal, onMount } from 'solid-js';
import { invoke } from '@tauri-apps/api/core';
import './App.css';

function App() {
  const [count, setCount] = createSignal(0);
  const [status, setStatus] = createSignal('Idle');

  onMount(() => {
    // Listen for state updates from Crux Core via Tauri IPC
    // This makes the UI a "reactive observer" of the Rust state machine
    console.log('WarmStreet SolidJS app mounted');
  });

  const handleReport = async () => {
    setStatus('Reporting');
    try {
      // Dispatch an Event to the Crux State Machine in Rust
      await invoke('process_event', { event: { type: 'ReportSpotted' } });
      setStatus('Reported');
    } catch (error) {
      console.error('Failed to report:', error);
      setStatus('Error');
    }
  };

  return (
    <div class="app">
      <header class="app-header">
        <h1>WarmStreet</h1>
        <p class="tagline">Zero-Latency Animal Rescue Coordination</p>
        <p class="subtitle">SolidJS + Tauri + Crux = Zero Jank</p>
      </header>

      <main class="app-main">
        <div class="card">
          <p>
            Welcome to WarmStreet — the world's first zero-latency animal rescue
            coordination platform.
          </p>
          <p class="info-text">
            Built with SolidJS for fine-grained reactivity. No virtual DOM overhead.
            Pure signal-based updates for native-level performance.
          </p>
        </div>

        <div class="card">
          <button onClick={handleReport} class="rescue-button">
            {status() === 'Reporting' ? 'Sending...' : status() === 'Reported' ? 'Reported!' : 'Report Animal'}
          </button>
          <p class="status-text">Status: {status()}</p>
        </div>

        <div class="card">
          <button onClick={() => setCount((c) => c + 1)}>
            count is {count()}
          </button>
          <p>
            Edit <code>src/App.tsx</code> and save to test HMR updates.
          </p>
          <p class="info-text">
            SolidJS updates only the affected text node - no re-renders, no wasted cycles.
          </p>
        </div>
      </main>

      <footer class="app-footer">
        <p>Not Trash. Every line of code exists to eliminate a specific failure.</p>
      </footer>
    </div>
  );
}

export default App;
