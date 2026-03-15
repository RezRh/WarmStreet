import { createSignal } from 'solid-js';
import { ShieldCheck, ArrowRight, RefreshCw } from 'lucide-solid';

interface MfaVerificationPageProps {
  onVerify: (code: string) => Promise<void>;
  onCancel: () => void;
  error?: string;
}

export const MfaVerificationPage = (props: MfaVerificationPageProps) => {
  const [code, setCode] = createSignal('');
  const [loading, setLoading] = createSignal(false);
  const [localError, setLocalError] = createSignal('');

  const handleSubmit = async (e: Event) => {
    e.preventDefault();
    if (code().length < 6) {
      setLocalError('Please enter a valid 6-digit code');
      return;
    }

    setLoading(true);
    setLocalError('');
    try {
      await props.onVerify(code());
    } catch (err: any) {
      setLocalError(err.message || 'Verification failed');
    } finally {
      setLoading(false);
    }
  };

  return (
    <div class="fixed inset-0 z-[60] bg-zinc-950/95 backdrop-blur-xl flex flex-col items-center justify-center p-6 font-geist">
      <div class="w-full max-w-md animate-in fade-in zoom-in duration-300">
        <div class="bg-zinc-900/50 border border-white/10 rounded-[2.5rem] p-8 md:p-10 shadow-2xl">
          <div class="flex flex-col items-center text-center gap-6">
            <div class="w-20 h-20 rounded-3xl bg-violet-500/20 flex items-center justify-center border border-violet-500/30 shadow-[0_0_30px_rgba(139,92,246,0.2)]">
              <ShieldCheck class="w-10 h-10 text-violet-400" />
            </div>
            
            <div class="space-y-2">
              <h1 class="text-3xl font-bold text-white">Two-Factor Auth</h1>
              <p class="text-zinc-400 text-sm max-w-[280px]">
                Enter the 6-digit verification code from your authenticator app or email.
              </p>
            </div>

            <form onSubmit={handleSubmit} class="w-full space-y-6">
              <div class="relative group">
                <input
                  type="text"
                  inputMode="numeric"
                  pattern="[0-9]*"
                  maxLength={6}
                  placeholder="000000"
                  value={code()}
                  onInput={(e) => setCode(e.currentTarget.value.replace(/\D/g, ''))}
                  class="w-full bg-zinc-950/50 border border-zinc-800 text-center text-4xl tracking-[0.5em] font-mono py-6 rounded-2xl focus:outline-none focus:border-violet-500/50 focus:ring-4 focus:ring-violet-500/10 transition-all placeholder:text-zinc-800 placeholder:tracking-normal"
                  autofocus
                />
                <div class="absolute inset-0 rounded-2xl bg-gradient-to-tr from-violet-500/5 to-transparent opacity-0 group-focus-within:opacity-100 pointer-events-none transition-opacity"></div>
              </div>

              {(props.error || localError()) && (
                <p class="text-red-400 text-sm animate-in fade-in slide-in-from-top-2">
                  {props.error || localError()}
                </p>
              )}

              <button
                type="submit"
                disabled={loading()}
                class="w-full group relative overflow-hidden rounded-2xl bg-violet-600 py-4 font-bold text-white hover:bg-violet-500 active:scale-[0.98] transition-all shadow-[0_0_30px_rgba(139,92,246,0.3)] disabled:opacity-50"
              >
                <div class="flex items-center justify-center gap-2">
                  {loading() ? (
                    <RefreshCw class="w-5 h-5 animate-spin" />
                  ) : (
                    <>
                      Verify Code
                      <ArrowRight class="w-5 h-5 group-hover:translate-x-1 transition-transform" />
                    </>
                  )}
                </div>
              </button>
            </form>

            <button
              onClick={props.onCancel}
              class="text-zinc-500 hover:text-zinc-300 text-sm font-medium transition-colors"
            >
              Cancel and Sign In again
            </button>
          </div>
        </div>
        
        <p class="mt-8 text-center text-zinc-600 text-xs uppercase tracking-widest font-semibold">
          SECURED BY APPWRITE
        </p>
      </div>
    </div>
  );
};
