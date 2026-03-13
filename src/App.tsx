import { createSignal, onMount } from 'solid-js';
import { listen, invoke } from '@tauri-apps/api/core';
import './App.css';

// ViewModel from Crux Rust core
interface ViewModel {
  status: 'Loading' | 'Unauthenticated' | 'Ready' | 'Error';
  feed_view: 'map' | 'list';
  cases: Case[];
  selected_case: Case | null;
  is_refreshing: boolean;
  error: string | null;
  toast: string | null;
  // ... add all ViewModel fields from Rust
}

interface Case {
  id: string;
  location: { lat: number; lon: number };
  description: string;
  status: string;
  // ... add all Case fields from Rust
}

const initial_view_model: ViewModel = {
  status: 'Loading',
  feed_view: 'map',
  cases: [],
  selected_case: null,
  is_refreshing: false,
  error: null,
  toast: null,
};

function App() {
  const [view_model, set_view_model] = createSignal<ViewModel>(initial_view_model);

  onMount(() => {
    // Listen for ViewModel updates from Crux Rust core
    // This is the ONLY way UI gets state updates
    const unlisten = listen('crux-update', (event: any) => {
      console.log('ViewModel update:', event.payload);
      set_view_model(event.payload);
    });

    // Dispatch initial event to start app
    invoke('dispatch_event', { event: 'AppStarted' });

    return () => {
      unlisten.then(f => f());
    };
  });

  // UI just dispatches events - no logic
  const dispatch = (event: string, payload?: any) => {
    console.log('Dispatching event:', event, payload);
    invoke('dispatch_event', { event, payload });
  };

  return (
    <div class="app">
      {/* Header - displays state from ViewModel */}
      <header class="app-header">
        <h1>WarmStreet</h1>
        <p class="tagline">Zero-Latency Animal Rescue Coordination</p>
        {view_model().is_refreshing && <span class="loading">Refreshing...</span>}
      </header>

      {/* Error Display - from ViewModel */}
      {view_model().error && (
        <div class="error-banner">
          {view_model().error}
          <button onClick={() => dispatch('DismissError')}>×</button>
        </div>
      )}

      {/* Toast - from ViewModel */}
      {view_model().toast && (
        <div class="toast">
          {view_model().toast}
        </div>
      )}

      <main class="app-main">
        {/* Status-based rendering - all logic in Rust */}
        {view_model().status === 'Loading' && (
          <div class="loading-screen">Loading...</div>
        )}

        {view_model().status === 'Unauthenticated' && (
          <div class="login-screen">
            <button onClick={() => dispatch('LoginRequested')}>
              Login
            </button>
          </div>
        )}

        {view_model().status === 'Ready' && (
          <>
            {/* Feed View Toggle - state from Rust */}
            <div class="feed-toggle">
              <button 
                class={view_model().feed_view === 'map' ? 'active' : ''}
                onClick={() => dispatch('SwitchToMap')}
              >
                Map
              </button>
              <button 
                class={view_model().feed_view === 'list' ? 'active' : ''}
                onClick={() => dispatch('SwitchToList')}
              >
                List
              </button>
            </div>

            {/* Feed Display - data from Rust */}
            {view_model().feed_view === 'map' ? (
              <div class="map-feed">
                {/* Map component - cases from view_model().cases */}
                <p>Map view with {view_model().cases.length} cases</p>
              </div>
            ) : (
              <div class="list-feed">
                {/* List component - cases from view_model().cases */}
                {view_model().cases.map(case_ => (
                  <div class="case-card" key={case_.id}>
                    <h3>{case_.description}</h3>
                    <p>Status: {case_.status}</p>
                    <button onClick={() => dispatch('CaseSelected', { case_id: case_.id })}>
                      View Details
                    </button>
                  </div>
                ))}
              </div>
            )}

            {/* Report Button - just dispatches event */}
            <button 
              class="report-button"
              onClick={() => dispatch('ReportSpotted')}
            >
              Report Animal
            </button>
          </>
        )}
      </main>

      <footer class="app-footer">
        <p>Not Trash. Every line of code exists to eliminate a specific failure.</p>
      </footer>
    </div>
  );
}

export default App;
