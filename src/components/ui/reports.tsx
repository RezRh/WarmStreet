import { createSignal, For, Show } from 'solid-js';
import { 
  Search, 
  MapPin, 
  Clock, 
  Info,
  Stethoscope,
  Activity,
  Zap
} from 'lucide-solid';

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
  weight?: string;
  symptoms?: string[];
  confidence?: string;
}

interface ReportsPageProps {
  cases: Case[];
  onCaseSelect: (id: string) => void;
}

export const ReportsPage = (props: ReportsPageProps) => {
  const [activeStatus, setActiveStatus] = createSignal('All');
  const [activeSeverity, setActiveSeverity] = createSignal('All');
  const [isSearchOpen, setIsSearchOpen] = createSignal(false);
  const [searchQuery, setSearchQuery] = createSignal('');

  const statusOptions = ['All', 'Pending', 'In Progress', 'Resolved'];
  const severityOptions = ['All', 'Critical', 'High', 'Moderate', 'Low'];

  return (
    <div class="flex flex-col h-full space-y-6 pt-4">
      {/* Search and Title Row */}
      <div class="relative min-h-[80px]">
        <Show when={!isSearchOpen()}>
          <div class="flex justify-between items-end animate-in fade-in slide-in-from-left-4 duration-300">
            <div>
              <h1 class="text-4xl font-black tracking-tight text-white mb-1">Reports</h1>
              <p class="text-zinc-500 text-xs font-bold uppercase tracking-widest">
                {props.cases.length} Rescues Found
              </p>
            </div>
            <button 
              onClick={() => setIsSearchOpen(true)}
              class="w-12 h-12 rounded-2xl bg-white/[0.03] border border-white/10 flex items-center justify-center hover:bg-white/[0.08] transition-all active:scale-95"
            >
              <Search class="w-5 h-5 text-zinc-400" />
            </button>
          </div>
        </Show>

        <Show when={isSearchOpen()}>
          <div class="flex items-center gap-3 animate-in fade-in slide-in-from-right-4 duration-300">
            <div class="flex-1 relative">
              <div class="absolute left-4 top-1/2 -translate-y-1/2">
                <Search class="w-4 h-4 text-zinc-500" />
              </div>
              <input 
                autofocus
                type="text" 
                placeholder="Search cases..." 
                value={searchQuery()}
                onInput={(e) => setSearchQuery(e.currentTarget.value)}
                class="w-full bg-white/[0.03] border border-white/10 rounded-2xl py-4 pl-12 pr-4 text-sm font-medium text-white focus:outline-none focus:border-white/20 transition-all placeholder:text-zinc-600"
              />
            </div>
            <button 
              onClick={() => {
                setIsSearchOpen(false);
                setSearchQuery('');
              }}
              class="px-4 py-4 rounded-2xl bg-white/5 text-xs font-bold text-zinc-400 hover:text-white transition-colors"
            >
              Cancel
            </button>
          </div>
        </Show>
      </div>

      {/* Filter Sections */}
      <div class="space-y-4">
        <div class="flex flex-col gap-2">
          <span class="text-[10px] font-black text-zinc-600 uppercase tracking-[0.2em] ml-1">Status</span>
          <div class="flex gap-2 overflow-x-auto pb-1 no-scrollbar">
            <For each={statusOptions}>
              {(status) => (
                <button 
                  onClick={() => setActiveStatus(status)}
                  class={`px-5 py-2.5 rounded-full text-xs font-bold whitespace-nowrap transition-all border ${
                    activeStatus() === status 
                    ? 'bg-white text-black border-white' 
                    : 'bg-zinc-900/50 text-zinc-500 border-white/5 hover:border-white/20'
                  }`}
                >
                  {status}
                </button>
              )}
            </For>
          </div>
        </div>

        <div class="flex flex-col gap-2">
          <span class="text-[10px] font-black text-zinc-600 uppercase tracking-[0.2em] ml-1">Severity</span>
          <div class="flex gap-2 overflow-x-auto pb-1 no-scrollbar">
            <For each={severityOptions}>
              {(sev) => (
                <button 
                  onClick={() => setActiveSeverity(sev)}
                  class={`px-5 py-2.5 rounded-full text-xs font-bold whitespace-nowrap transition-all border ${
                    activeSeverity() === sev 
                    ? 'bg-white text-black border-white shadow-[0_0_15px_rgba(255,255,255,0.2)]' 
                    : 'bg-zinc-900/50 text-zinc-500 border-white/5 hover:border-white/20'
                  }`}
                >
                  {sev}
                </button>
              )}
            </For>
          </div>
        </div>
      </div>

      {/* Reports List */}
      <div class="flex flex-col gap-6 pb-20">
        <For each={props.cases}>
          {(item) => (
            <div 
              onClick={() => props.onCaseSelect(item.id)}
              class="group relative bg-zinc-900/40 border border-white/5 rounded-[2.5rem] overflow-hidden backdrop-blur-md hover:bg-zinc-900/60 transition-all active:scale-[0.99] shadow-2xl"
            >
              {/* Glass Reflection effect */}
              <div class="absolute inset-x-0 top-0 h-px bg-gradient-to-r from-transparent via-white/20 to-transparent pointer-events-none" />
              
              {/* Image Header */}
              <div class="relative h-64 overflow-hidden">
                <img 
                  src={item.imageUrl} 
                  class="w-full h-full object-cover transition-transform duration-700 group-hover:scale-110" 
                  alt={item.description}
                />
                
                {/* Badges Overlay */}
                <div class="absolute inset-0 bg-gradient-to-t from-zinc-950/80 via-transparent to-transparent" />
                
                <div class="absolute top-4 left-4 flex gap-2">
                  <div class={`px-3 py-1.5 rounded-full text-[10px] font-black uppercase tracking-widest flex items-center gap-1.5 backdrop-blur-xl border border-white/20 shadow-lg ${
                    item.severity === 'Critical' ? 'bg-red-500/80 text-white' : 
                    (item.severity === 'High' || item.severity === 'Moderate') ? 'bg-violet-600/80 text-white border-violet-400/20' : 
                    'bg-zinc-800/80 text-zinc-300'
                  }`}>
                    <Activity class="w-3 h-3" />
                    {item.severity}
                  </div>
                </div>

                <div class="absolute top-4 right-4">
                  <div class="px-3 py-1.5 rounded-full bg-white text-black text-[10px] font-black uppercase tracking-widest shadow-lg">
                    {item.status.replace('_', ' ')}
                  </div>
                </div>

                <div class="absolute bottom-4 right-4">
                  <div class="px-3 py-1.5 rounded-full bg-black/40 backdrop-blur-md border border-white/10 text-[10px] font-bold text-white flex items-center gap-1.5">
                    <Clock class="w-3 h-3 text-zinc-400" />
                    4h ago
                  </div>
                </div>
              </div>

              {/* Content Section */}
              <div class="p-6 space-y-4">
                <h3 class="text-xl font-bold leading-tight group-hover:text-violet-400 transition-colors">
                  {item.description}
                </h3>

                <div class="flex flex-wrap gap-y-2 gap-x-4 items-center text-zinc-400">
                  <div class="flex items-center gap-2">
                    <div class="w-8 h-8 rounded-full bg-white/5 flex items-center justify-center">
                      <Zap class="w-4 h-4 text-white" />
                    </div>
                    <span class="text-xs font-bold text-zinc-300 uppercase tracking-wide">{item.type} • {item.breed}</span>
                  </div>
                  <div class="flex items-center gap-2 text-xs font-medium opacity-60">
                    <Info class="w-3.5 h-3.5" />
                    <span>Adult • 200-250 kg</span>
                  </div>
                </div>

                <p class="text-sm text-zinc-500 leading-relaxed line-clamp-2">
                  The primary health concern is severe, chronic malnutrition (cachexia) occurring concurrently with potential infectious or systemic pathologies...
                </p>

                {/* Tags Section */}
                <div class="flex flex-wrap gap-2 pt-2">
                  <div class="flex items-center gap-2 px-4 py-2 bg-white/5 border border-white/10 rounded-xl text-[10px] font-bold text-zinc-300 uppercase tracking-widest">
                    <MapPin class="w-3 h-3" />
                    View Map
                  </div>
                  <div class="flex items-center gap-2 px-4 py-2 bg-white/5 border border-white/10 rounded-xl text-[10px] font-bold text-zinc-300 uppercase tracking-widest">
                    <Stethoscope class="w-3 h-3" />
                    6 Symptoms
                  </div>
                  <div class="flex items-center gap-2 px-4 py-2 bg-white/5 border border-white/10 rounded-xl text-[10px] font-bold text-zinc-300 uppercase tracking-widest">
                    <Activity class="w-3 h-3" />
                    Lethargic
                  </div>
                </div>

                {/* AI Confidence */}
                <div class="pt-4 flex items-center justify-between border-t border-white/5">
                  <div class="flex items-center gap-2">
                    <Activity class="w-3.5 h-3.5 text-zinc-600" />
                    <span class="text-[10px] font-black text-zinc-600 uppercase tracking-widest">AI Confidence</span>
                  </div>
                  <span class="text-xs font-black text-white">98%</span>
                </div>
              </div>
            </div>
          )}
        </For>
      </div>
    </div>
  );
};
