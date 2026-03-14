import { createSignal, Show } from 'solid-js';
import { Building2, User, ArrowRight, Stethoscope } from 'lucide-solid';
import { account } from '../../lib/appwrite';

interface VerificationPageProps {
  onComplete: (metadata: any) => void;
  onSignOut: () => void;
}

export const VerificationPage = (props: VerificationPageProps) => {
  const [userType, setUserType] = createSignal<'individual' | 'ngo' | 'vet'>('individual');
  const [loading, setLoading] = createSignal(false);
  const [error, setError] = createSignal('');

  const handleSubmit = async (e: Event) => {
    e.preventDefault();
    setLoading(true);
    setError('');

    const form = e.target as HTMLFormElement;
    const formData = new FormData(form);
    
    const metadata: any = { user_type: userType() };
    if (userType() === 'ngo') {
      metadata.org_name = formData.get('org_name');
      metadata.reg_number = formData.get('reg_number');
      metadata.phone = formData.get('phone');
    } else if (userType() === 'vet') {
      metadata.clinic_name = formData.get('clinic_name');
      metadata.license_number = formData.get('license_number');
      metadata.phone = formData.get('phone');
    }

    try {
      await account.updatePrefs(metadata);
      props.onComplete(metadata);
    } catch (err: any) {
      setError(err.message);
    } finally {
      setLoading(false);
    }
  };

  return (
    <div class="fixed inset-0 z-50 bg-zinc-950 flex flex-col items-center justify-start p-6 pt-[calc(4rem + env(safe-area-inset-top, 24px))] overflow-y-auto overflow-x-hidden">
      <div class="w-full max-w-sm">
        <h1 class="text-3xl font-bold mb-2">Complete Profile</h1>
        <p class="text-zinc-400 mb-8">Please select your account type to continue.</p>

        <form onSubmit={handleSubmit} class="space-y-6">
          <div class="grid grid-cols-3 gap-2 p-1 bg-zinc-900/50 backdrop-blur-md rounded-2xl border border-white/5">
            <button
              type="button"
              onClick={() => setUserType('individual')}
              class={`flex flex-col items-center justify-center gap-1 py-3 rounded-xl transition-all ${
                userType() === 'individual' ? 'bg-zinc-800 text-white shadow-lg' : 'text-zinc-500 hover:text-zinc-300'
              }`}
            >
              <User class="w-4 h-4" />
              <span class="text-[10px] font-bold uppercase tracking-wider">Individual</span>
            </button>
            <button
              type="button"
              onClick={() => setUserType('ngo')}
              class={`flex flex-col items-center justify-center gap-1 py-3 rounded-xl transition-all ${
                userType() === 'ngo' ? 'bg-zinc-800 text-white shadow-lg' : 'text-zinc-500 hover:text-zinc-300'
              }`}
            >
              <Building2 class="w-4 h-4" />
              <span class="text-[10px] font-bold uppercase tracking-wider">NGO</span>
            </button>
            <button
              type="button"
              onClick={() => setUserType('vet')}
              class={`flex flex-col items-center justify-center gap-1 py-3 rounded-xl transition-all ${
                userType() === 'vet' ? 'bg-zinc-800 text-white shadow-lg' : 'text-zinc-500 hover:text-zinc-300'
              }`}
            >
              <Stethoscope class="w-4 h-4" />
              <span class="text-[10px] font-bold uppercase tracking-wider">Vet</span>
            </button>
          </div>

          <Show when={userType() === 'ngo'}>
            <div class="space-y-4 animate-in fade-in slide-in-from-top-4 duration-500">
              <div>
                <label class="text-xs font-medium text-zinc-500 uppercase tracking-wider mb-1.5 block">Organization Name</label>
                <div class="bg-zinc-900/50 border border-white/10 rounded-2xl p-1 focus-within:border-violet-500/50 transition-colors">
                  <input name="org_name" type="text" placeholder="Rescue Angels NGO" class="w-full bg-transparent text-sm p-4 rounded-2xl focus:outline-none" required />
                </div>
              </div>
              <div class="grid grid-cols-2 gap-4">
                <div>
                  <label class="text-xs font-medium text-zinc-500 uppercase tracking-wider mb-1.5 block">Reg. Number</label>
                  <div class="bg-zinc-900/50 border border-white/10 rounded-2xl p-1 focus-within:border-violet-500/50 transition-colors">
                    <input name="reg_number" type="text" placeholder="REG123" class="w-full bg-transparent text-sm p-4 rounded-2xl focus:outline-none" required />
                  </div>
                </div>
                <div>
                  <label class="text-xs font-medium text-zinc-500 uppercase tracking-wider mb-1.5 block">Phone</label>
                  <div class="bg-zinc-900/50 border border-white/10 rounded-2xl p-1 focus-within:border-violet-500/50 transition-colors">
                    <input name="phone" type="tel" placeholder="+1234..." class="w-full bg-transparent text-sm p-4 rounded-2xl focus:outline-none" required />
                  </div>
                </div>
              </div>
            </div>
          </Show>

          <Show when={userType() === 'vet'}>
            <div class="space-y-4 animate-in fade-in slide-in-from-top-4 duration-500">
              <div>
                <label class="text-xs font-medium text-zinc-500 uppercase tracking-wider mb-1.5 block">Clinic / Hospital Name</label>
                <div class="bg-zinc-900/50 border border-white/10 rounded-2xl p-1 focus-within:border-violet-500/50 transition-colors">
                  <input name="clinic_name" type="text" placeholder="Paws & Claws Clinic" class="w-full bg-transparent text-sm p-4 rounded-2xl focus:outline-none" required />
                </div>
              </div>
              <div class="grid grid-cols-2 gap-4">
                <div>
                  <label class="text-xs font-medium text-zinc-500 uppercase tracking-wider mb-1.5 block">License Number</label>
                  <div class="bg-zinc-900/50 border border-white/10 rounded-2xl p-1 focus-within:border-violet-500/50 transition-colors">
                    <input name="license_number" type="text" placeholder="VET-789-01" class="w-full bg-transparent text-sm p-4 rounded-2xl focus:outline-none" required />
                  </div>
                </div>
                <div>
                  <label class="text-xs font-medium text-zinc-500 uppercase tracking-wider mb-1.5 block">Work Phone</label>
                  <div class="bg-zinc-900/50 border border-white/10 rounded-2xl p-1 focus-within:border-violet-500/50 transition-colors">
                    <input name="phone" type="tel" placeholder="+1234..." class="w-full bg-transparent text-sm p-4 rounded-2xl focus:outline-none" required />
                  </div>
                </div>
              </div>
            </div>
          </Show>

          {error() && <p class="text-red-400 text-sm text-center">{error()}</p>}

          <button
            type="submit"
            disabled={loading()}
            class="w-full rounded-2xl bg-violet-600 py-4 font-semibold text-white hover:bg-violet-500 active:scale-[0.98] transition-all shadow-[0_0_20px_rgba(139,92,246,0.3)] disabled:opacity-50 flex items-center justify-center gap-2"
          >
            {loading() ? 'Saving...' : 'Complete Registration'}
            <ArrowRight class="w-5 h-5" />
          </button>
        </form>

        <button
          onClick={props.onSignOut}
          class="w-full mt-6 text-sm text-zinc-500 hover:text-zinc-300 transition-colors"
        >
          Sign Out & Start Over
        </button>
      </div>
    </div>
  );
};
