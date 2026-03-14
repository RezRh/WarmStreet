import { For, Show, createSignal } from 'solid-js';
import { Search, MapPin, MessageSquare, Phone, Info, Building2, Stethoscope } from 'lucide-solid';

export interface CommunityMember {
  id: string;
  name: string;
  member_type: 'Vet' | 'NGO';
  description: string;
  location_name: string;
  phone: string;
  image_url: string;
  lat: number;
  lon: number;
}

interface CommunityPageProps {
  members: CommunityMember[];
  isLoading: boolean;
  onRefresh: () => void;
  onMessage: (id: string) => void;
}

export const CommunityPage = (props: CommunityPageProps) => {
  const [filter, setFilter] = createSignal<'All' | 'Vet' | 'NGO'>('All');
  const [searchQuery, setSearchQuery] = createSignal('');

  const filteredMembers = () => {
    let result = props.members;
    if (filter() !== 'All') {
      result = result.filter(m => m.member_type === filter());
    }
    if (searchQuery().trim() !== '') {
      const q = searchQuery().toLowerCase();
      result = result.filter(m => 
        m.name.toLowerCase().includes(q) || 
        m.location_name.toLowerCase().includes(q)
      );
    }
    return result;
  };

  const handleText = (e: MouseEvent, phone: string, memberId: string) => {
    e.stopPropagation();
    props.onMessage(memberId);
    // Trigger native SMS
    window.location.href = `sms:${phone}`;
  };

  return (
    <div class="flex flex-col h-full bg-zinc-950 text-white font-sans animate-in fade-in duration-500">
      {/* Header Section */}
      <div class="px-6 pt-6 pb-4 space-y-4">
        <div class="flex justify-between items-end">
          <div class="space-y-1">
            <h2 class="text-3xl font-bold tracking-tight">Community</h2>
            <p class="text-zinc-500 text-sm font-medium">Find help for rescues nearby</p>
          </div>
          <div class="px-3 py-1 bg-violet-600/10 border border-violet-500/20 rounded-full text-[10px] font-bold text-violet-400 uppercase tracking-widest animate-pulse">
            Local Network
          </div>
        </div>

        {/* Search Bar */}
        <div class="relative group">
          <div class="absolute inset-y-0 left-4 flex items-center pointer-events-none">
            <Search class="w-4 h-4 text-zinc-500 group-focus-within:text-violet-400 transition-colors" />
          </div>
          <input 
            type="text" 
            placeholder="Search vets, clinics, NGOs..."
            onInput={(e) => setSearchQuery(e.currentTarget.value)}
            class="w-full bg-zinc-900/50 border border-white/5 rounded-2xl py-3.5 pl-11 pr-4 text-sm focus:outline-none focus:border-violet-500/30 focus:bg-zinc-900/80 transition-all placeholder:text-zinc-600"
          />
        </div>

        {/* Filter Chips */}
        <div class="flex gap-2.5 overflow-x-auto no-scrollbar py-1">
          <For each={['All', 'Vet', 'NGO'] as const}>
            {(type) => (
              <button 
                onClick={() => setFilter(type)}
                class={`px-5 py-2.5 rounded-full text-xs font-bold transition-all border whitespace-nowrap active:scale-95 ${
                  filter() === type 
                    ? 'bg-white text-zinc-950 border-white shadow-[0_0_20px_rgba(255,255,255,0.1)]' 
                    : 'bg-zinc-900/50 border-white/5 text-zinc-500 hover:text-zinc-300 hover:bg-zinc-900'
                }`}
              >
                {type === 'All' ? 'Everywhere' : type === 'Vet' ? 'Veterinarians' : 'NGO Shelters'}
              </button>
            )}
          </For>
        </div>
      </div>

      {/* Results List */}
      <div class="flex-1 overflow-y-auto px-6 pb-32 space-y-4 custom-scrollbar">
        <Show when={props.isLoading && filteredMembers().length === 0}>
          <div class="flex flex-col items-center justify-center py-20 space-y-4">
            <div class="w-12 h-12 border-2 border-violet-500/20 border-t-violet-500 rounded-full animate-spin" />
            <span class="text-zinc-500 text-sm font-medium">Scanning local networks...</span>
          </div>
        </Show>

        <Show when={!props.isLoading && filteredMembers().length === 0}>
          <div class="flex flex-col items-center justify-center py-20 text-center space-y-2 opacity-60">
            <div class="w-16 h-16 bg-zinc-900 rounded-full flex items-center justify-center mb-2">
              <Search class="w-8 h-8 text-zinc-700" />
            </div>
            <h3 class="font-bold text-white">No results found</h3>
            <p class="text-xs text-zinc-500 px-10">Try adjusting your search or filter to find more community members.</p>
          </div>
        </Show>

        <For each={filteredMembers()}>
          {(member) => (
            <div class="group bg-zinc-900/40 border border-white/5 rounded-[2rem] p-5 space-y-4 active:scale-[0.98] transition-all relative overflow-hidden backdrop-blur-md hover:bg-zinc-900/60 hover:border-white/10">
              {/* Type Badge Overlay */}
              <div class={`absolute top-0 right-0 px-6 py-1.5 rounded-bl-[1.5rem] text-[9px] font-black uppercase tracking-[0.2em] shadow-lg ${
                member.member_type === 'Vet' 
                  ? 'bg-blue-600/90 text-white' 
                  : 'bg-violet-600/90 text-white'
              }`}>
                {member.member_type}
              </div>

              <div class="flex gap-4">
                {/* Profile Image/Avatar */}
                <div class="relative shrink-0">
                  <div class="w-20 h-20 rounded-2xl overflow-hidden bg-zinc-800 border border-white/10 shadow-inner">
                    <img 
                      src={member.image_url} 
                      alt={member.name} 
                      class="w-full h-full object-cover grayscale opacity-80 group-hover:grayscale-0 group-hover:opacity-100 transition-all duration-500" 
                    />
                  </div>
                  <div class="absolute -bottom-2 -right-2 p-2 bg-zinc-950 rounded-xl border border-white/10 shadow-xl">
                    <Show when={member.member_type === 'Vet'}>
                      <Stethoscope class="w-3.5 h-3.5 text-blue-400" />
                    </Show>
                    <Show when={member.member_type === 'NGO'}>
                      <Building2 class="w-3.5 h-3.5 text-violet-400" />
                    </Show>
                  </div>
                </div>

                {/* Main Info */}
                <div class="flex-1 space-y-2">
                  <div class="pr-12">
                    <h3 class="font-bold text-lg leading-tight text-white group-hover:text-violet-400 transition-colors uppercase tracking-tight">
                      {member.name}
                    </h3>
                  </div>
                  
                  <div class="flex items-center gap-1.5 text-zinc-400">
                    <MapPin class="w-3 h-3 text-zinc-600" />
                    <span class="text-[11px] font-bold uppercase tracking-wider">{member.location_name}</span>
                  </div>

                  <p class="text-[12px] leading-relaxed text-zinc-500 line-clamp-2 font-medium italic">
                    "{member.description}"
                  </p>
                </div>
              </div>

              {/* Action Buttons */}
              <div class="grid grid-cols-2 gap-3 pt-2">
                <button 
                  onClick={(e) => handleText(e, member.phone, member.id)}
                  class="flex items-center justify-center gap-2 py-3.5 px-4 bg-white text-zinc-950 rounded-2xl font-bold text-xs active:scale-95 transition-all shadow-xl hover:bg-zinc-200"
                >
                  <MessageSquare class="w-3.5 h-3.5 fill-current" />
                  Direct Text
                </button>
                <button 
                  onClick={(e) => {
                    e.stopPropagation();
                    window.location.href = `tel:${member.phone}`;
                  }}
                  class="flex items-center justify-center gap-2 py-3.5 px-4 bg-zinc-800/50 border border-white/5 text-white rounded-2xl font-bold text-xs active:scale-95 transition-all hover:bg-zinc-800 hover:border-white/10"
                >
                  <Phone class="w-3.5 h-3.5 text-zinc-400 group-hover:text-white" />
                  Call Clinic
                </button>
              </div>

              {/* Subtle Progress Bar-like indicator for reliability or distance */}
              <div class="absolute bottom-0 left-0 h-0.5 bg-violet-600/30 transition-all duration-700 w-0 group-hover:w-full" />
            </div>
          )}
        </For>

        {/* Info Card */}
        <div class="bg-gradient-to-br from-violet-600/10 to-transparent border border-violet-500/10 rounded-[2rem] p-6 text-center space-y-3">
          <div class="flex justify-center">
            <Info class="w-8 h-8 text-violet-500/50" />
          </div>
          <h4 class="text-sm font-bold text-white uppercase tracking-widest">Emergency Network</h4>
          <p class="text-xs text-zinc-500 font-medium px-4">These vets and NGOs are community-verified. Always call first to ensure availability of specialized care.</p>
        </div>
      </div>
    </div>
  );
};
