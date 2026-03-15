import { Show, For, createSignal, onMount } from 'solid-js';
import { 
  User, 
  Settings, 
  Shield, 
  Phone, 
  Mail,
  Award,
  ChevronRight,
  Fingerprint,
  Activity,
  Zap,
  Star
} from 'lucide-solid';
import { account } from '../../lib/appwrite';
import { SettingsModal } from './settings';

interface ProfilePageProps {
  onSignOut: () => void;
}

export const ProfilePage = (props: ProfilePageProps) => {
  const [user, setUser] = createSignal<any>(null);
  const [loading, setLoading] = createSignal(true);
  const [isSettingsOpen, setIsSettingsOpen] = createSignal(false);

  onMount(async () => {
    try {
      const currentUser = await account.get();
      setUser(currentUser);
    } catch (err) {
      console.log('ℹ️ No Appwrite session, assuming guest mode');
      // Mock guest user data for design preview
      setUser({
        name: 'Rezwanur Rahman',
        email: 'rezwan@warmstreet.org',
        prefs: {
          user_type: 'vet',
          phone: '+91 98765 43210'
        }
      });
    } finally {
      setTimeout(() => setLoading(false), 800); // Smooth transition
    }
  });

  const getUserTypeLabel = () => {
    const type = user()?.prefs?.user_type;
    if (type === 'individual') return 'Elite Volunteer';
    if (type === 'ngo') return 'Authorized NGO';
    if (type === 'vet') return 'Certified Veterinarian';
    return 'Community Member';
  };

  const getUserTypeColor = () => {
    const type = user()?.prefs?.user_type;
    if (type === 'ngo') return 'from-indigo-500 via-purple-500 to-pink-500';
    if (type === 'vet') return 'from-cyan-500 via-blue-600 to-indigo-600';
    return 'from-emerald-400 via-teal-500 to-cyan-600';
  };

  return (
    <div class="flex flex-col h-full bg-zinc-950 text-white font-sans overflow-hidden selection:bg-white/10">
      <Show when={loading()}>
        <div class="absolute inset-0 z-[100] flex items-center justify-center bg-zinc-950">
          <div class="relative w-24 h-24">
            <div class={`absolute inset-0 rounded-full border-4 border-white/5 border-t-white/40 animate-spin`} />
            <div class="absolute inset-4 rounded-full bg-white/5 backdrop-blur-xl flex items-center justify-center">
              <Zap class="w-6 h-6 text-white animate-pulse" />
            </div>
          </div>
        </div>
      </Show>

      {/* Dynamic Background Glow */}
      <div class="absolute top-0 left-1/2 -translate-x-1/2 w-[150%] h-[50%] pointer-events-none opacity-20">
        <div class={`absolute inset-0 bg-gradient-to-b ${getUserTypeColor()} blur-[120px] rounded-full`} />
      </div>

      <SettingsModal 
        isOpen={isSettingsOpen()} 
        onClose={() => setIsSettingsOpen(false)} 
        onSignOut={props.onSignOut} 
      />

      {/* Main Scrollable Area */}
      <div class="flex-1 overflow-y-auto px-6 pt-16 pb-40 no-scrollbar relative z-10 space-y-8">
        
        {/* Profile Header Block */}
        <section class="flex flex-col items-center text-center space-y-6">
          <div class="relative group">
            {/* Liquid-style Halo */}
            <div class={`absolute -inset-4 bg-gradient-to-tr ${getUserTypeColor()} opacity-25 blur-2xl group-hover:opacity-40 transition-opacity duration-700 animate-pulse`} />
            
            <div class="relative">
              <div class={`p-[3px] rounded-[2.5rem] bg-gradient-to-tr ${getUserTypeColor()} shadow-2xl`}>
                <div class="w-28 h-28 rounded-[2.4rem] bg-zinc-950 flex items-center justify-center overflow-hidden border border-white/10">
                  <Show when={user()?.name} fallback={<User class="w-12 h-12 text-zinc-800" />}>
                    <span class="text-4xl font-black bg-gradient-to-br from-white to-white/30 bg-clip-text text-transparent select-none tracking-tighter">
                      {user()?.name?.substring(0, 1).toUpperCase()}
                    </span>
                  </Show>
                </div>
              </div>
              <div class="absolute -bottom-1 -right-1 w-9 h-9 bg-zinc-950 rounded-2xl border border-white/10 flex items-center justify-center shadow-2xl">
                <Star class="w-4 h-4 text-amber-400 fill-amber-400" />
              </div>
            </div>
          </div>

          <div class="space-y-2">
            <h1 class="text-3xl font-black tracking-tight text-white">{user()?.name || 'User'}</h1>
            <div class="inline-flex items-center gap-2 px-4 py-1.5 rounded-full bg-white/[0.03] border border-white/10 backdrop-blur-xl">
              <div class={`w-1.5 h-1.5 rounded-full bg-gradient-to-r ${getUserTypeColor()} shadow-[0_0_10px_rgba(255,255,255,0.3)] animate-pulse`} />
              <span class="text-[10px] font-black uppercase tracking-[0.2em] text-zinc-400">{getUserTypeLabel()}</span>
            </div>
          </div>
        </section>

        {/* Stats Bento Grid */}
        <section class="grid grid-cols-2 gap-4">
          <div class="bg-white/[0.03] border border-white/10 rounded-[2rem] p-5 space-y-1 backdrop-blur-3xl hover:bg-white/[0.05] transition-all group">
            <div class="flex justify-between items-start">
              <div class="p-2.5 rounded-xl bg-violet-500/10 border border-violet-500/20">
                <Activity class="w-4 h-4 text-violet-400" />
              </div>
              <span class="text-[10px] font-bold text-emerald-400">+12%</span>
            </div>
            <div class="pt-2">
              <div class="text-2xl font-black group-hover:scale-105 transition-transform origin-left">142</div>
              <div class="text-[10px] font-bold text-zinc-500 uppercase tracking-widest">Global Rescue Karma</div>
            </div>
          </div>

          <div class="bg-white/[0.03] border border-white/10 rounded-[2rem] p-5 space-y-1 backdrop-blur-3xl hover:bg-white/[0.05] transition-all group">
            <div class="flex justify-between items-start">
              <div class="p-2.5 rounded-xl bg-blue-500/10 border border-blue-500/20">
                <Shield class="w-4 h-4 text-blue-400" />
              </div>
              <span class="text-[10px] font-bold text-blue-400">SR 1</span>
            </div>
            <div class="pt-2">
              <div class="text-2xl font-black group-hover:scale-105 transition-transform origin-left">Elite</div>
              <div class="text-[10px] font-bold text-zinc-500 uppercase tracking-widest">Verification Level</div>
            </div>
          </div>
        </section>

        {/* Activity Heatmap - Bespoke Element */}
        <section class="space-y-4">
          <div class="flex items-center justify-between px-2">
            <h3 class="text-[11px] font-black text-zinc-600 uppercase tracking-[0.3em]">Rescue Frequency</h3>
            <div class="flex gap-1">
              <div class="w-2 h-2 rounded-full bg-emerald-500/20" />
              <div class="w-2 h-2 rounded-full bg-emerald-500/50" />
              <div class="w-2 h-2 rounded-full bg-emerald-500" />
            </div>
          </div>
          <div class="bg-white/[0.02] border border-white/5 rounded-[2rem] p-6 backdrop-blur-md">
            <div class="flex justify-between items-end h-20 gap-1.5">
              <For each={[40, 70, 45, 90, 65, 30, 85, 50, 95, 60, 40, 75]}>
                {(height: number) => (
                  <div class="flex-1 group/bar relative">
                    <div 
                      class="w-full bg-white/5 rounded-full transition-all duration-500 group-hover/bar:bg-white/20" 
                      style={{ height: `${height}%`, "background-color": height > 80 ? 'rgba(52, 211, 153, 0.3)' : '' }}
                    />
                    <div class="absolute -bottom-6 left-1/2 -translate-x-1/2 text-[8px] font-bold text-zinc-700 opacity-0 group-hover/bar:opacity-100 transition-opacity">
                      M1
                    </div>
                  </div>
                )}
              </For>
            </div>
            <div class="pt-8 flex justify-between text-[10px] font-black text-zinc-600 uppercase tracking-widest">
              <span>Jan</span>
              <span>Jun</span>
              <span>Dec</span>
            </div>
          </div>
        </section>

        {/* Interaction List */}
        <section class="space-y-4">
          <div class="flex items-center justify-between px-2">
            <h3 class="text-[11px] font-black text-zinc-600 uppercase tracking-[0.3em]">Security & Identity</h3>
            <Fingerprint class="w-4 h-4 text-zinc-800" />
          </div>
          
          <div class="space-y-3">
            <div class="group bg-white/[0.03] border border-white/10 rounded-3xl px-6 py-5 flex items-center gap-5 hover:bg-white/[0.06] transition-all cursor-pointer">
              <div class="w-12 h-12 rounded-2xl bg-zinc-900 border border-white/10 flex items-center justify-center group-hover:rotate-6 transition-transform">
                <Mail class="w-5 h-5 text-zinc-400" />
              </div>
              <div class="flex-1">
                <div class="text-[10px] font-black text-zinc-600 uppercase tracking-widest mb-0.5">Primary Corridor</div>
                <div class="text-sm font-bold text-zinc-200">{user()?.email || 'N/A'}</div>
              </div>
              <ChevronRight class="w-5 h-5 text-zinc-800 group-hover:text-white group-hover:translate-x-1 transition-all" />
            </div>

            <div class="group bg-white/[0.03] border border-white/10 rounded-3xl px-6 py-5 flex items-center gap-5 hover:bg-white/[0.06] transition-all cursor-pointer">
              <div class="w-12 h-12 rounded-2xl bg-zinc-900 border border-white/10 flex items-center justify-center group-hover:rotate-6 transition-transform">
                <Phone class="w-5 h-5 text-zinc-400" />
              </div>
              <div class="flex-1">
                <div class="text-[10px] font-black text-zinc-600 uppercase tracking-widest mb-0.5">Secure Contact</div>
                <div class="text-sm font-bold text-zinc-200">{user()?.prefs?.phone || '+91 0000 0000'}</div>
              </div>
              <ChevronRight class="w-5 h-5 text-zinc-800 group-hover:text-white group-hover:translate-x-1 transition-all" />
            </div>
          </div>
        </section>

        {/* Merit System - Floating Badges */}
        <section class="space-y-4">
          <div class="flex items-center justify-between px-2">
            <h3 class="text-[11px] font-black text-zinc-600 uppercase tracking-[0.3em]">Merit System</h3>
            <Award class="w-4 h-4 text-zinc-800" />
          </div>
          
          <div class="grid grid-cols-3 gap-3">
            <div class="bg-gradient-to-tr from-white/[0.04] to-transparent border border-white/10 rounded-3xl p-5 flex flex-col items-center gap-3 group hover:border-amber-500/30 transition-all">
              <div class="w-12 h-12 rounded-2xl bg-amber-500/10 border border-amber-500/20 flex items-center justify-center group-hover:scale-110 transition-transform shadow-[0_0_20px_rgba(245,158,11,0.1)]">
                <Award class="w-6 h-6 text-amber-500" />
              </div>
              <span class="text-[9px] font-black text-zinc-400 uppercase tracking-tighter">Responder</span>
            </div>

            <div class="bg-gradient-to-tr from-white/[0.04] to-transparent border border-white/10 rounded-3xl p-5 flex flex-col items-center gap-3 group grayscale opacity-40 hover:grayscale-0 hover:opacity-100 transition-all">
              <div class="w-12 h-12 rounded-2xl bg-blue-500/10 border border-blue-500/20 flex items-center justify-center group-hover:scale-110 transition-transform">
                <Zap class="w-6 h-6 text-blue-400" />
              </div>
              <span class="text-[9px] font-black text-zinc-400 uppercase tracking-tighter">Speedster</span>
            </div>

            <div class="bg-gradient-to-tr from-white/[0.04] to-transparent border border-white/10 rounded-3xl p-5 flex flex-col items-center gap-3 group grayscale opacity-40 hover:grayscale-0 hover:opacity-100 transition-all">
              <div class="w-12 h-12 rounded-2xl bg-emerald-500/10 border border-emerald-500/20 flex items-center justify-center group-hover:scale-110 transition-transform">
                <Shield class="w-6 h-6 text-emerald-400" />
              </div>
              <span class="text-[9px] font-black text-zinc-400 uppercase tracking-tighter">Guardian</span>
            </div>
          </div>
        </section>

        {/* Action Tray */}
        <section class="flex flex-col gap-3 pt-4">
          <button 
            onClick={() => setIsSettingsOpen(true)}
            class="w-full bg-white text-zinc-950 py-5 rounded-[2rem] font-black text-xs uppercase tracking-[0.2em] flex items-center justify-center gap-3 active:scale-95 transition-all shadow-[0_20px_40px_rgba(255,255,255,0.1)] hover:bg-zinc-200"
          >
            <Settings class="w-4 h-4" />
            System Preferences
          </button>
        </section>

        <div class="text-center pb-8 opacity-20">
          <span class="text-[9px] font-black uppercase tracking-[0.4em]">WS-CORE V1.0.42_STABLE</span>
        </div>
      </div>
    </div>
  );
};
