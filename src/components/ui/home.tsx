import { createSignal, For, Show, createEffect, onMount, onCleanup } from 'solid-js';
import { Bell, MapPin, RefreshCcw, Search } from 'lucide-solid';
import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import { LiquidNav } from './liquid-nav';
import { ReportsPage } from './reports';
import { CommunityPage } from './community-page';

interface Case {
  id: string;
  description: string;
  status: string;
  severity: 'Low' | 'Moderate' | 'High' | 'Critical';
  type: string;
  age: string;
  breed: string;
  imageUrl: string;
  date: string;
}

interface HomePageProps {
  cases: Case[];
  communityMembers: any[];
  isLoadingCommunity: boolean;
  onReport: () => void;
  onRefresh: () => void;
  onCaseSelect: (id: string) => void;
  onMessageMember: (id: string) => void;
  onSignOut: () => void;
}

export const HomePage = (props: HomePageProps) => {
  const [activeTab, setActiveTab] = createSignal('home');
  const [searchRadius, setSearchRadius] = createSignal(5);
  
  createEffect(() => {
    if (activeTab() === 'community' && props.communityMembers.length === 0) {
      console.log('🏘️ Switching to Community - requesting data');
      invoke('handle_event', { event: 'CommunityRequested' });
    }
  });

  createEffect(() => console.log('Current Active Tab in HomePage:', activeTab()));
  
  onMount(() => {
    console.log('🗺️ home.tsx: onMount — triggering SwitchToMap');
    // Tell Crux to switch to map mode, which triggers the shell to show the native view
    invoke('handle_event', { event: 'SwitchToMap' });

    // Listen for pin taps from the native layer
    const unlistenPin = listen('crux-map-pin-tapped', (event) => {
      const caseId = event.payload as string;
      console.log('📍 home.tsx: pin tapped:', caseId);
      props.onCaseSelect(caseId);
    });

    // Listen for map ready to potentially trigger local UI updates
    const unlistenReady = listen('crux-map-ready', () => {
      console.log('🗺️ home.tsx: native map ready');
    });

    onCleanup(async () => {
      console.log('🗺️ home.tsx: onCleanup — triggering SwitchToList');
      invoke('handle_event', { event: 'SwitchToList' });
      (await unlistenPin)();
      (await unlistenReady)();
    });
  });

  const radiusOptions = [1, 2, 5, 10];

  return (
    <div class="flex flex-col h-screen bg-zinc-950 text-white overflow-hidden font-sans">
      {/* Header - Dynamic based on tab */}
      <Show when={activeTab() === 'home'}>
        <header class="px-6 pt-14 pb-4 flex flex-col gap-1 shrink-0">
          <div class="flex justify-between items-center">
            <div class="flex flex-col">
              <h1 class="text-3xl font-bold tracking-tight text-white">WarmStreet</h1>
            </div>
          <div class="flex items-center gap-2">
            <div class="flex items-center p-1 bg-white/[0.03] backdrop-blur-xl border border-white/10 rounded-full">
              <button class="w-10 h-10 rounded-full flex items-center justify-center hover:bg-white/5 transition-all active:scale-95">
                <Search class="w-5 h-5 text-white/70" />
              </button>
            </div>
            <div class="flex items-center p-1.5 bg-white/[0.05] backdrop-blur-[30px] saturate-[180%] border border-white/10 rounded-full shadow-lg relative overflow-hidden group">
              <div class="absolute inset-0 bg-white/5 pointer-events-none" style="filter: url(#liquid-filter)" />
              <button class="w-11 h-11 rounded-full flex items-center justify-center relative hover:bg-white/5 transition-all active:scale-95 z-10">
                <Bell class="w-5 h-5 text-white/90" />
                <span class="absolute top-2 right-2 w-4 h-4 bg-red-500 rounded-full text-[10px] font-bold flex items-center justify-center border-2 border-zinc-950 shadow-sm animate-pulse">8</span>
              </button>
            </div>
          </div>
          </div>
          <div class="flex items-center gap-2 text-zinc-400 mt-2">
            <MapPin class="w-4 h-4 text-zinc-500" />
            <span class="text-xs font-medium">Shop No 8 ramesh nagar gol cha...</span>
          </div>
        </header>
      </Show>

      {/* Scrollable Content */}
      <main class="flex-1 overflow-y-auto px-6 pb-32 custom-scrollbar">
        <Show when={activeTab() === 'home'}>
          <div class="space-y-6 pt-2">
            {/* Live Rescue Map Card */}
            <section class="bg-zinc-900/40 border border-white/5 rounded-[2rem] p-6 space-y-4 backdrop-blur-xl">
              <div class="flex justify-between items-center">
                <div class="space-y-1">
                  <h2 class="text-lg font-bold text-white">Live Rescue Map</h2>
                  <div class="flex items-center gap-3">
                    <span class="text-white text-sm font-bold">{props.cases.length} cases</span>
                    <div class="w-1 h-1 bg-zinc-700 rounded-full"></div>
                    <div class="flex items-center gap-1.5 text-zinc-400 text-[10px] font-bold uppercase tracking-widest">
                      <div class="w-1.5 h-1.5 bg-red-500 rounded-full animate-pulse" />
                      <span class="opacity-80">Native Bridge Active</span>
                    </div>
                  </div>
                </div>
                <button onClick={props.onRefresh} class="p-2 text-zinc-500 hover:text-white transition-colors">
                  <RefreshCcw class="w-5 h-5" />
                </button>
              </div>

              {/* Map Mockup — Made transparent for Native Map overlay */}
              <div class="relative h-48 w-full rounded-3xl overflow-hidden bg-transparent border border-white/10 shadow-inner group">
                {/* The native map is rendered BEHIND the webview. 
                    This div serves as a viewport. We keep the overlay elements 
                    but remove the static map image. */}
                
                <div class="absolute inset-0 bg-transparent pointer-events-none" />

                {/* Optional: A subtle gradient to help readability of overlays if the map is too bright */}
                <div class="absolute inset-0 bg-gradient-to-b from-zinc-950/20 to-transparent pointer-events-none" />

                {/* My Location Badge (remains in web layer for consistent styling) */}
                <div class="absolute top-4 left-4 flex items-center gap-2 px-3 py-1.5 bg-zinc-950/80 backdrop-blur-md rounded-full border border-white/10 shadow-lg z-10">
                  <div class="w-2 h-2 bg-blue-500 rounded-full animate-pulse" />
                  <span class="text-[10px] font-bold text-zinc-200">Live Map View</span>
                </div>
                
                {/* Native Bridge Indicator (top right) */}
                <div class="absolute top-4 right-4 flex items-center gap-1.5 px-3 py-1.5 bg-zinc-950/80 backdrop-blur-md rounded-full border border-white/10 shadow-lg z-10">
                  <div class="w-1.5 h-1.5 bg-red-500 rounded-full animate-pulse" />
                  <span class="text-[10px] font-bold text-white/70 uppercase tracking-tighter">Native</span>
                </div>

                {/* Hint for developers */}
                <div class="absolute inset-0 flex items-center justify-center pointer-events-none opacity-0 group-hover:opacity-100 transition-opacity">
                  <span class="text-[10px] text-white/20 font-mono">[Native Map Content Underneath]</span>
                </div>
              </div>

              {/* Radius Selector */}
              <div class="space-y-3">
                <span class="text-xs font-bold text-zinc-500 uppercase tracking-widest">Search Radius</span>
                <div class="flex gap-2">
                  <For each={radiusOptions}>
                    {(radius) => (
                      <button 
                        onClick={() => setSearchRadius(radius)}
                        class={`flex-1 py-3 rounded-2xl text-sm font-bold transition-all border active:scale-95 ${
                          searchRadius() === radius 
                            ? 'bg-white/10 border-white/20 text-white shadow-[0_0_15px_rgba(255,255,255,0.05)]' 
                            : 'bg-zinc-900 border-white/5 text-zinc-500 hover:text-zinc-300 hover:bg-zinc-800'
                        }`}
                      >
                        {radius} km
                      </button>
                    )}
                  </For>
                </div>
              </div>
            </section>

            {/* Emergency Cases Section */}
            <section class="space-y-4">
              <div class="flex justify-between items-center">
                <h2 class="text-xl font-bold text-white">Emergency Cases ({props.cases.length})</h2>
                <button onClick={props.onRefresh} class="p-2 text-zinc-500 hover:text-white transition-colors">
                  <RefreshCcw class="w-5 h-5" />
                </button>
              </div>

              <div class="flex gap-4 overflow-x-auto pb-4 -mx-2 px-2 custom-scrollbar snap-x">
                <For each={props.cases}>
                  {(item) => (
                    <div 
                      onClick={() => props.onCaseSelect(item.id)}
                      class="snap-start min-w-[280px] bg-zinc-900/40 border border-white/5 rounded-[2rem] p-4 space-y-3 active:scale-[0.98] transition-all cursor-pointer backdrop-blur-md hover:bg-zinc-900/60 hover:border-white/10 group"
                    >
                      <div class="relative h-32 rounded-2xl overflow-hidden bg-zinc-800">
                        <img src={item.imageUrl} alt={item.description} class="w-full h-full object-cover transition-transform duration-500 group-hover:scale-105" />
                        <div class={`absolute top-3 right-3 px-3 py-1 backdrop-blur-md rounded-full border border-white/10 text-[10px] font-bold ${
                          item.severity === 'High' || item.severity === 'Moderate' 
                          ? 'bg-violet-600/80 text-white border-violet-400/30' 
                          : 'bg-zinc-950/80 text-zinc-300'
                        }`}>
                          {item.severity}
                        </div>
                      </div>
                      <div class="px-1 space-y-1">
                        <div class="flex justify-between items-start">
                          <h3 class="font-bold text-base truncate pr-2 text-white group-hover:text-violet-400 transition-colors">{item.description}</h3>
                        </div>
                        <p class="text-[10px] font-medium text-zinc-500 uppercase tracking-wider">
                          {item.type} • {item.age} • {item.breed}
                        </p>
                        <div class="flex items-center gap-1.5 pt-2">
                          <div class={`px-3 py-1.5 rounded-xl text-[10px] font-bold ${
                            item.status === 'IN PROGRESS' ? 'bg-blue-500/10 text-blue-400 border border-blue-500/20' : 'bg-emerald-500/10 text-emerald-400 border border-emerald-500/20'
                          }`}>
                            {item.status.replace('_', ' ')}
                          </div>
                          <div class="flex items-center gap-1 text-zinc-500 text-[10px] font-bold uppercase tracking-widest ml-auto">
                            <span class="opacity-60">Posted:</span>
                            <span>{item.date}</span>
                          </div>
                        </div>
                      </div>
                    </div>
                  )}
                </For>
              </div>
            </section>
          </div>
        </Show>

        <Show when={activeTab() === 'reports'}>
          <div class="pt-14">
            <ReportsPage cases={props.cases as any} onCaseSelect={props.onCaseSelect} />
          </div>
        </Show>

        <Show when={activeTab() === 'community'}>
          <div class="h-full">
            <CommunityPage 
              members={props.communityMembers} 
              isLoading={props.isLoadingCommunity}
              onRefresh={() => invoke('handle_event', { event: 'CommunityRequested' })}
              onMessage={props.onMessageMember}
            />
          </div>
        </Show>

        <Show when={activeTab() !== 'home' && activeTab() !== 'reports' && activeTab() !== 'community'}>
          <div class="flex items-center justify-center h-full text-zinc-500">
            Tab: {activeTab()} (Coming Soon)
          </div>
        </Show>
      </main>

      {/* Liquid Glass Navigation */}
      <LiquidNav 
        activeTab={activeTab()} 
        onTabChange={(tab) => setActiveTab(tab)} 
        onReport={props.onReport}
      />

      {/* Bottom spacer for safe area */}
      <div class="h-8 w-full" />
    </div>
  );
};
