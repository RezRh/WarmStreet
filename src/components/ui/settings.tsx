import { Show, createSignal } from 'solid-js';
import { 
  X, 
  Trash2, 
  ShieldCheck, 
  Sun, 
  Moon, 
  LogOut, 
  ChevronRight,
  Monitor
} from 'lucide-solid';

interface SettingsModalProps {
  isOpen: boolean;
  onClose: () => void;
  onSignOut: () => void;
}

export const SettingsModal = (props: SettingsModalProps) => {
  const [theme, setTheme] = createSignal<'light' | 'dark' | 'system'>('dark');

  const handleToggleTheme = () => {
    const nextTheme = theme() === 'dark' ? 'light' : 'dark';
    setTheme(nextTheme);
    // In a real app, this would update documentElement.classList or a theme context
    if (nextTheme === 'light') {
      document.documentElement.classList.add('light-mode');
    } else {
      document.documentElement.classList.remove('light-mode');
    }
  };

  return (
    <Show when={props.isOpen}>
      <div class="fixed inset-0 z-[200] flex items-end justify-center sm:items-center px-4 pb-10">
        {/* Backdrop */}
        <div 
          class="absolute inset-0 bg-zinc-950/80 backdrop-blur-md transition-opacity duration-300"
          onClick={props.onClose}
        />

        {/* Modal Sheet */}
        <div class="relative w-full max-w-lg bg-zinc-900/90 border border-white/10 rounded-[2.5rem] overflow-hidden shadow-2xl animate-in slide-in-from-bottom-20 duration-500 backdrop-blur-3xl">
          {/* Header */}
          <div class="px-8 pt-8 pb-4 flex justify-between items-center bg-gradient-to-b from-white/[0.02] to-transparent">
            <h2 class="text-2xl font-black tracking-tight text-white">Settings</h2>
            <button 
              onClick={props.onClose}
              class="w-10 h-10 rounded-2xl bg-white/5 border border-white/10 flex items-center justify-center hover:bg-white/10 transition-all active:scale-90"
            >
              <X class="w-5 h-5 text-zinc-400" />
            </button>
          </div>

          <div class="p-4 space-y-6">
            {/* Preferences Section */}
            <section class="space-y-3">
              <h3 class="px-4 text-[10px] font-black text-zinc-600 uppercase tracking-[0.3em]">Preferences</h3>
              
              <div class="bg-white/[0.03] border border-white/5 rounded-3xl overflow-hidden">
                <button 
                  onClick={handleToggleTheme}
                  class="w-full px-6 py-5 flex items-center justify-between hover:bg-white/[0.05] transition-all group"
                >
                  <div class="flex items-center gap-4">
                    <div class="w-10 h-10 rounded-xl bg-amber-500/10 flex items-center justify-center">
                      <Show when={theme() === 'dark'} fallback={<Sun class="w-5 h-5 text-amber-500" />}>
                        <Moon class="w-5 h-5 text-amber-400" />
                      </Show>
                    </div>
                    <div class="text-left">
                      <div class="text-sm font-bold text-white">Interface Theme</div>
                      <div class="text-[10px] font-bold text-zinc-500 uppercase tracking-widest">Current: {theme() === 'dark' ? 'Midnight' : 'Daylight'}</div>
                    </div>
                  </div>
                  <div class="w-12 h-6 bg-zinc-800 rounded-full relative p-1 transition-colors">
                    <div class={`w-4 h-4 rounded-full bg-white transition-all transform ${theme() === 'dark' ? 'translate-x-6' : 'translate-x-0'}`} />
                  </div>
                </button>
              </div>
            </section>

            {/* Legal Section */}
            <section class="space-y-3">
              <h3 class="px-4 text-[10px] font-black text-zinc-600 uppercase tracking-[0.3em]">Legal & Privacy</h3>
              
              <div class="bg-white/[0.03] border border-white/5 rounded-3xl overflow-hidden">
                <button class="w-full px-6 py-5 flex items-center justify-between hover:bg-white/[0.05] transition-all group border-b border-white/5">
                  <div class="flex items-center gap-4">
                    <div class="w-10 h-10 rounded-xl bg-violet-500/10 flex items-center justify-center">
                      <ShieldCheck class="w-5 h-5 text-violet-400" />
                    </div>
                    <div class="text-left font-bold text-zinc-200">Privacy Policy</div>
                  </div>
                  <ChevronRight class="w-5 h-5 text-zinc-700 group-hover:text-white group-hover:translate-x-1 transition-all" />
                </button>

                <button class="w-full px-6 py-5 flex items-center justify-between hover:bg-white/[0.05] transition-all group">
                  <div class="flex items-center gap-4">
                    <div class="w-10 h-10 rounded-xl bg-zinc-500/10 flex items-center justify-center">
                      <Monitor class="w-5 h-5 text-zinc-400" />
                    </div>
                    <div class="text-left font-bold text-zinc-200">Terms of Service</div>
                  </div>
                  <ChevronRight class="w-5 h-5 text-zinc-700 group-hover:text-white group-hover:translate-x-1 transition-all" />
                </button>
              </div>
            </section>

            {/* Danger Zone */}
            <section class="space-y-3 pb-4">
              <h3 class="px-4 text-[10px] font-black text-red-500/50 uppercase tracking-[0.3em]">Critical Operations</h3>
              
              <div class="bg-red-500/[0.03] border border-red-500/10 rounded-3xl overflow-hidden">
                <button 
                  onClick={() => {
                    props.onSignOut();
                    props.onClose();
                  }}
                  class="w-full px-6 py-5 flex items-center justify-between hover:bg-red-500/10 transition-all group border-b border-red-500/10"
                >
                  <div class="flex items-center gap-4">
                    <div class="w-10 h-10 rounded-xl bg-red-500/10 flex items-center justify-center">
                      <LogOut class="w-5 h-5 text-red-400" />
                    </div>
                    <div class="text-left font-bold text-red-400/80 group-hover:text-red-400">Terminate Session</div>
                  </div>
                  <ChevronRight class="w-5 h-5 text-red-900 group-hover:text-red-400 group-hover:translate-x-1 transition-all" />
                </button>

                <button class="w-full px-6 py-5 flex items-center justify-between hover:bg-red-500/20 transition-all group">
                  <div class="flex items-center gap-4">
                    <div class="w-10 h-10 rounded-xl bg-red-500/20 flex items-center justify-center">
                      <Trash2 class="w-5 h-5 text-red-500" />
                    </div>
                    <div class="text-left font-bold text-red-500">Delete Account</div>
                  </div>
                  <span class="text-[9px] font-black text-red-900 uppercase tracking-widest px-3 py-1 bg-red-500/10 rounded-lg">Permanent</span>
                </button>
              </div>
            </section>
          </div>
          
          <div class="px-8 py-6 text-center border-t border-white/5 bg-zinc-950/20">
            <p class="text-[9px] font-black text-zinc-600 uppercase tracking-[0.4em]">Auth-Stream v1.0.42 // Secure Node</p>
          </div>
        </div>
      </div>
    </Show>
  );
};
