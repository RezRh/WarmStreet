import { For } from 'solid-js';
import { 
  Home, 
  Heart, 
  Users, 
  User,
  Camera
} from 'lucide-solid';

interface LiquidNavProps {
  activeTab: string;
  onTabChange: (tab: string) => void;
  onReport: () => void;
}

export const LiquidNav = (props: LiquidNavProps) => {
  const tabs = [
    { id: 'home', icon: Home, label: 'Home' },
    { id: 'reports', icon: Heart, label: 'Reports' },
    { id: 'community', icon: Users, label: 'Community' },
    { id: 'profile', icon: User, label: 'Profile' },
  ];

  return (
    <div class="fixed bottom-8 left-1/2 -translate-x-1/2 flex items-center gap-4 z-50">
      {/* SVG Filters (Hidden) */}
      <svg class="absolute invisible w-0 h-0">
        <filter id="liquid-filter" color-interpolation-filters="sRGB">
          <feGaussianBlur in="SourceGraphic" stdDeviation="6" result="blur" />
          <feColorMatrix in="blur" type="matrix" values="1 0 0 0 0  0 1 0 0 0  0 0 1 0 0  0 0 0 18 -7" result="goo" />
          <feComposite in="SourceGraphic" in2="goo" operator="atop" />
        </filter>
      </svg>

      {/* Main Container - More reflective glass */}
      <div class="flex items-center gap-3 p-2 bg-white/[0.08] backdrop-blur-[50px] saturate-[200%] border border-white/20 rounded-[2.5rem] shadow-[0_12px_40px_rgba(0,0,0,0.4)] relative overflow-hidden group">
        
        {/* Gooey Background Layer - Highly reflective white glow */}
        <div class="absolute inset-0 pointer-events-none" style="filter: url(#liquid-filter)">
          <For each={tabs}>
            {(tab) => (
              <div 
                class={`absolute top-2 w-12 h-12 bg-white/40 rounded-2xl transition-all duration-500 ease-out opacity-0 scale-75 ${
                  props.activeTab === tab.id ? 'opacity-100 scale-100' : ''
                }`}
                style={{
                  left: `${tabs.indexOf(tab) * 3.5 + 0.5}rem`,
                }}
              />
            )}
          </For>
        </div>

        {/* Icons Layer with Enhanced Hover/Active States */}
        <div class="flex gap-2 relative z-10">
          <For each={tabs}>
            {(tab) => (
              <button
                onClick={() => {
                  console.log('Nav item clicked:', tab.id);
                  // alert('Clicked: ' + tab.id); // Uncomment if console is hard to reach
                  props.onTabChange(tab.id);
                }}
                class={`w-12 h-12 flex items-center justify-center rounded-2xl transition-all duration-300 active:scale-90 group relative z-20 cursor-pointer ${
                  props.activeTab === tab.id ? 'text-white' : 'text-zinc-500 hover:text-zinc-200'
                }`}
              >
                {/* Visual hover indicator */}
                <div class={`absolute inset-0 rounded-2xl bg-white/5 opacity-0 group-hover:opacity-100 transition-opacity duration-200 ${props.activeTab === tab.id ? 'hidden' : ''}`} />
                
                <tab.icon class={`w-6 h-6 z-10 transition-transform duration-300 ${props.activeTab === tab.id ? 'scale-110' : 'group-hover:scale-110'}`} />
              </button>
            )}
          </For>
        </div>
      </div>

      {/* Separate FAB - High Quality Reflection */}
      <button 
        onClick={props.onReport}
        class="w-14 h-14 rounded-full bg-gradient-to-tr from-white/10 to-white/20 backdrop-blur-[50px] saturate-[200%] border border-white/20 shadow-[0_12px_40px_rgba(0,0,0,0.4)] flex items-center justify-center active:scale-90 transition-all hover:bg-white/25 group relative overflow-hidden"
      >
        <div class="absolute inset-0 rounded-full bg-white/5 opacity-0 group-hover:opacity-100 transition-opacity" />
        <Camera class="w-7 h-7 text-white/90 transition-transform group-hover:scale-110 z-10" />
      </button>
    </div>
  );
};
