import { createSignal, Show } from 'solid-js';
import { Eye, EyeOff } from 'lucide-solid';

// --- HELPER COMPONENTS (ICONS) ---

const GoogleIcon = () => (
    <svg xmlns="http://www.w3.org/2000/svg" class="h-5 w-5" viewBox="0 0 48 48">
        <path fill="#FFC107" d="M43.611 20.083H42V20H24v8h11.303c-1.649 4.657-6.08 8-11.303 8-6.627 0-12-5.373-12-12s12-5.373 12-12c3.059 0 5.842 1.154 7.961 3.039l5.657-5.657C34.046 6.053 29.268 4 24 4 12.955 4 4 12.955 4 24s8.955 20 20 20 20-8.955 20-20c0-2.641-.21-5.236-.611-7.743z" />
        <path fill="#FF3D00" d="M6.306 14.691l6.571 4.819C14.655 15.108 18.961 12 24 12c3.059 0 5.842 1.154 7.961 3.039l5.657-5.657C34.046 6.053 29.268 4 24 4 16.318 4 9.656 8.337 6.306 14.691z" />
        <path fill="#4CAF50" d="M24 44c5.166 0 9.86-1.977 13.409-5.192l-6.19-5.238C29.211 35.091 26.715 36 24 36c-5.202 0-9.619-3.317-11.283-7.946l-6.522 5.025C9.505 39.556 16.227 44 24 44z" />
        <path fill="#1976D2" d="M43.611 20.083H42V20H24v8h11.303c-.792 2.237-2.231 4.166-4.087 5.571l6.19 5.238C42.022 35.026 44 30.038 44 24c0-2.641-.21-5.236-.611-7.743z" />
    </svg>
);

const AppleIcon = () => (
    <svg xmlns="http://www.w3.org/2000/svg" class="h-5 w-5" viewBox="0 0 384 512" fill="currentColor">
        <path d="M318.7 268.7c-.2-36.7 16.4-64.4 50-84.8-18.8-26.9-47.2-41.7-84.7-44.6-35.5-2.8-74.3 20.7-88.5 20.7-15 0-49.4-19.7-76.4-19.7C63.3 141.2 4 184.8 4 273.5q0 39.3 14.4 81.2c12.8 36.7 59 126.7 107.2 125.2 25.2-.6 43-17.9 75.8-17.9 31.8 0 48.3 17.9 76.4 17.9 48.6-.7 90.4-82.5 102.6-119.3-65.2-30.7-61.7-90-61.7-91.9zm-56.6-164.2c27.3-32.4 24.8-61.9 24-72.5-24.1 1.4-52 16.4-67.9 34.9-17.5 19.8-27.8 44.3-25.6 71.9 26.1 2 49.9-11.4 69.5-34.3z"/>
    </svg>
);


// --- TYPE DEFINITIONS ---

export interface Testimonial {
  avatarSrc: string;
  name: string;
  handle: string;
  text: string;
}

export interface SignInPageProps {
  title?: any;
  description?: any;
  heroImageSrc?: string;
  testimonials?: Testimonial[];
  onSignIn?: (event: Event) => void;
  onSignup?: (event: Event) => void;
  onGoogleSignIn?: () => void;
  onAppleSignIn?: () => void;
  onResetPassword?: () => void;
  onCreateAccount?: () => void;
  onContinueAsGuest?: () => void;
  mode?: 'signin' | 'signup';
}

// --- SUB-COMPONENTS ---

const GlassInputWrapper = (props: { children: any }) => (
  <div class="rounded-2xl border border-border bg-foreground/5 backdrop-blur-sm transition-colors focus-within:border-violet-400/70 focus-within:bg-violet-500/10">
    {props.children}
  </div>
);

const TestimonialCard = (props: { testimonial: Testimonial, delay: string }) => (
  <div class={`animate-testimonial ${props.delay} flex items-start gap-3 rounded-3xl bg-card/40 dark:bg-zinc-800/40 backdrop-blur-xl border border-white/10 p-5 w-64`}>
    <img src={props.testimonial.avatarSrc} class="h-10 w-10 object-cover rounded-2xl" alt="avatar" />
    <div class="text-sm leading-snug">
      <p class="flex items-center gap-1 font-medium">{props.testimonial.name}</p>
      <p class="text-muted-foreground">{props.testimonial.handle}</p>
      <p class="mt-1 text-foreground/80">{props.testimonial.text}</p>
    </div>
  </div>
);

// --- MAIN COMPONENT ---

export const SignInPage = (props: SignInPageProps) => {
  const [showPassword, setShowPassword] = createSignal(false);
  const [isSignup, setIsSignup] = createSignal(props.mode === 'signup');
  const [userType, setUserType] = createSignal<'individual' | 'ngo' | 'vet'>('individual');

  const handleSubmit = (e: Event) => {
    e.preventDefault();
    if (isSignup() && props.onSignup) {
      props.onSignup(e);
    } else if (props.onSignIn) {
      props.onSignIn(e);
    }
  };

  const toggleMode = () => {
    setIsSignup(!isSignup());
    if (isSignup() && props.onCreateAccount) {
      props.onCreateAccount();
    }
  };

  return (
    <div class="h-[100dvh] flex flex-col md:flex-row font-geist w-[100dvw] bg-zinc-950 text-white overflow-hidden pb-[env(safe-area-inset-bottom,20px)]">
      {/* Sign-in form scrollable container */}
      <section class="flex-1 flex flex-col items-center justify-start p-6 overflow-y-auto custom-scrollbar pt-[calc(3rem + env(safe-area-inset-top, 24px))]">
        <div class="w-full max-w-sm py-8">
          <div class="flex flex-col gap-6">
            <h1 class="animate-element animate-delay-100 text-4xl md:text-5xl font-semibold leading-tight text-white">
              {props.title || (isSignup() ? 'Create Account' : 'Welcome Back')}
            </h1>
            <p class="animate-element animate-delay-200 text-zinc-400">
              {props.description || (isSignup() ? 'Join the fastest animal rescue network' : 'Sign in to continue saving animals')}
            </p>

            {/* User Type Toggle for Signup */}
            <Show when={isSignup()}>
              <div class="animate-element animate-delay-250 p-1 bg-zinc-900 rounded-2xl grid grid-cols-3 gap-1">
                <button 
                  type="button"
                  onClick={() => setUserType('individual')}
                  class={`flex flex-col items-center justify-center gap-1 py-3 rounded-xl transition-all ${userType() === 'individual' ? 'bg-zinc-800 text-white shadow-lg' : 'text-zinc-500 hover:text-zinc-300'}`}
                >
                  <span class="text-[10px] font-bold uppercase tracking-wider">Individual</span>
                </button>
                <button 
                  type="button"
                  onClick={() => setUserType('ngo')}
                  class={`flex flex-col items-center justify-center gap-1 py-3 rounded-xl transition-all ${userType() === 'ngo' ? 'bg-zinc-800 text-white shadow-lg' : 'text-zinc-500 hover:text-zinc-300'}`}
                >
                  <span class="text-[10px] font-bold uppercase tracking-wider">NGO</span>
                </button>
                <button 
                  type="button"
                  onClick={() => setUserType('vet')}
                  class={`flex flex-col items-center justify-center gap-1 py-3 rounded-xl transition-all ${userType() === 'vet' ? 'bg-zinc-800 text-white shadow-lg' : 'text-zinc-500 hover:text-zinc-300'}`}
                >
                  <span class="text-[10px] font-bold uppercase tracking-wider">Vet</span>
                </button>
              </div>
            </Show>

            <form class="space-y-5" onSubmit={handleSubmit}>
              <input type="hidden" name="user_type" value={userType()} />
              
              {/* Name field - only for signup */}
              <Show when={isSignup()}>
                <div class="animate-element animate-delay-300">
                  <label class="text-xs font-medium text-zinc-500 uppercase tracking-wider mb-1.5 block">Full Name</label>
                  <GlassInputWrapper>
                    <input name="name" type="text" placeholder="John Doe" class="w-full bg-transparent text-sm p-4 rounded-2xl focus:outline-none" required />
                  </GlassInputWrapper>
                </div>
              </Show>

              {/* NGO Specific Fields */}
              <Show when={isSignup() && userType() === 'ngo'}>
                <div class="animate-element animate-delay-350 grid grid-cols-1 gap-5">
                  <div>
                    <label class="text-xs font-medium text-zinc-500 uppercase tracking-wider mb-1.5 block">Organization Name</label>
                    <GlassInputWrapper>
                      <input name="org_name" type="text" placeholder="Rescue Angels NGO" class="w-full bg-transparent text-sm p-4 rounded-2xl focus:outline-none" required />
                    </GlassInputWrapper>
                  </div>
                  <div class="grid grid-cols-2 gap-3">
                    <div>
                      <label class="text-xs font-medium text-zinc-500 uppercase tracking-wider mb-1.5 block">Reg. Number</label>
                      <GlassInputWrapper>
                        <input name="reg_number" type="text" placeholder="REG123" class="w-full bg-transparent text-sm p-4 rounded-2xl focus:outline-none" required />
                      </GlassInputWrapper>
                    </div>
                    <div>
                      <label class="text-xs font-medium text-zinc-500 uppercase tracking-wider mb-1.5 block">Phone</label>
                      <GlassInputWrapper>
                        <input name="phone" type="tel" placeholder="+1234..." class="w-full bg-transparent text-sm p-4 rounded-2xl focus:outline-none" required />
                      </GlassInputWrapper>
                    </div>
                  </div>
                </div>
              </Show>

              {/* Vet Specific Fields */}
              <Show when={isSignup() && userType() === 'vet'}>
                <div class="animate-element animate-delay-350 grid grid-cols-1 gap-5">
                  <div>
                    <label class="text-xs font-medium text-zinc-500 uppercase tracking-wider mb-1.5 block">Clinic / Hospital Name</label>
                    <GlassInputWrapper>
                      <input name="clinic_name" type="text" placeholder="Paws & Claws Clinic" class="w-full bg-transparent text-sm p-4 rounded-2xl focus:outline-none" required />
                    </GlassInputWrapper>
                  </div>
                  <div class="grid grid-cols-2 gap-3">
                    <div>
                      <label class="text-xs font-medium text-zinc-500 uppercase tracking-wider mb-1.5 block">License Number</label>
                      <GlassInputWrapper>
                        <input name="license_number" type="text" placeholder="VET-789-01" class="w-full bg-transparent text-sm p-4 rounded-2xl focus:outline-none" required />
                      </GlassInputWrapper>
                    </div>
                    <div>
                      <label class="text-xs font-medium text-zinc-500 uppercase tracking-wider mb-1.5 block">Work Phone</label>
                      <GlassInputWrapper>
                        <input name="phone" type="tel" placeholder="+1234..." class="w-full bg-transparent text-sm p-4 rounded-2xl focus:outline-none" required />
                      </GlassInputWrapper>
                    </div>
                  </div>
                </div>
              </Show>

              <div class={`animate-element ${isSignup() ? 'animate-delay-400' : 'animate-delay-300'}`}>
                <label class="text-xs font-medium text-zinc-500 uppercase tracking-wider mb-1.5 block">Email Address</label>
                <GlassInputWrapper>
                  <input name="email" type="email" placeholder="john@example.com" class="w-full bg-transparent text-sm p-4 rounded-2xl focus:outline-none" required />
                </GlassInputWrapper>
              </div>

              <div class={`animate-element ${isSignup() ? 'animate-delay-450' : 'animate-delay-400'}`}>
                <label class="text-xs font-medium text-zinc-500 uppercase tracking-wider mb-1.5 block">Password</label>
                <GlassInputWrapper>
                  <div class="relative">
                    <input name="password" type={showPassword() ? 'text' : 'password'} placeholder="••••••••" class="w-full bg-transparent text-sm p-4 pr-12 rounded-2xl focus:outline-none" required />
                    <button type="button" onClick={() => setShowPassword(!showPassword())} class="absolute inset-y-0 right-3 flex items-center">
                      <Show when={showPassword()} fallback={<Eye class="w-5 h-5 text-zinc-500 hover:text-white transition-colors" />}>
                        <EyeOff class="w-5 h-5 text-zinc-500 hover:text-white transition-colors" />
                      </Show>
                    </button>
                  </div>
                </GlassInputWrapper>
              </div>

              <Show when={!isSignup()}>
                <div class="animate-element animate-delay-500 flex items-center justify-between text-xs">
                  <label class="flex items-center gap-2 cursor-pointer group">
                    <input type="checkbox" name="rememberMe" class="rounded border-zinc-700 bg-zinc-900 text-violet-500 focus:ring-violet-500 focus:ring-offset-zinc-950" />
                    <span class="text-zinc-400 group-hover:text-zinc-300 transition-colors">Remember me</span>
                  </label>
                  <a href="#" onClick={(e) => { e.preventDefault(); props.onResetPassword?.(); }} class="text-violet-400 hover:text-violet-300 transition-colors">Forgot password?</a>
                </div>
              </Show>

              <button type="submit" class={`animate-element ${isSignup() ? 'animate-delay-500' : 'animate-delay-600'} w-full rounded-2xl bg-violet-600 py-4 font-semibold text-white hover:bg-violet-500 active:scale-[0.98] transition-all shadow-[0_0_20px_rgba(139,92,246,0.3)]`}>
                {isSignup() ? 'Create Account' : 'Sign In'}
              </button>
            </form>

            <div class="animate-element animate-delay-700 relative flex items-center justify-center">
              <span class="w-full border-t border-zinc-900"></span>
              <span class="px-4 text-xs font-medium text-zinc-500 bg-zinc-950 absolute uppercase tracking-widest">Or continue with</span>
            </div>

            <div class="animate-element animate-delay-800 grid grid-cols-2 gap-4">
              <button 
                onClick={props.onGoogleSignIn} 
                class="flex items-center justify-center gap-3 bg-zinc-900 border border-zinc-800 rounded-2xl py-4 hover:bg-zinc-800 active:scale-[0.98] transition-all"
              >
                <GoogleIcon />
                <span class="text-sm font-medium">Google</span>
              </button>
              <button 
                onClick={props.onAppleSignIn} 
                class="flex items-center justify-center gap-3 bg-white text-black rounded-2xl py-4 hover:bg-zinc-200 active:scale-[0.98] transition-all"
              >
                <AppleIcon />
                <span class="text-sm font-medium">Apple</span>
              </button>
            </div>

            {/* Guest button - Hidden for NGOs and Vets */}
            <Show when={userType() === 'individual'}>
              <div class="animate-element animate-delay-900">
                <button onClick={props.onContinueAsGuest} class="w-full rounded-2xl bg-zinc-900 border border-zinc-800 py-4 font-semibold text-zinc-300 hover:bg-zinc-800 active:scale-[0.98] transition-all">
                  Browse as Guest
                </button>
              </div>
            </Show>

            <p class="animate-element animate-delay-1000 text-center text-sm text-zinc-500 mt-2">
              <Show when={isSignup()} fallback={<>New to WarmStreet? <a href="#" onClick={(e) => { e.preventDefault(); toggleMode(); }} class="text-violet-400 font-medium hover:text-violet-300 transition-colors">Create Account</a></>}>
                <>Already have an account? <a href="#" onClick={(e) => { e.preventDefault(); toggleMode(); }} class="text-violet-400 font-medium hover:text-violet-300 transition-colors">Sign In</a></>
              </Show>
            </p>
          </div>
        </div>
      </section>

      {/* Right column: hero image + testimonials (Hidden on small mobile) */}
      <Show when={props.heroImageSrc}>
        <section class="hidden lg:block flex-1 relative p-6 bg-zinc-900">
          <div class="animate-slide-right animate-delay-300 absolute inset-6 rounded-[2.5rem] bg-cover bg-center overflow-hidden grayscale-[0.2] contrast-[1.1]" style={{ "background-image": `url(${props.heroImageSrc})` }}>
            <div class="absolute inset-0 bg-gradient-to-t from-zinc-950 via-transparent to-transparent opacity-60"></div>
          </div>
          <Show when={props.testimonials && props.testimonials.length > 0}>
            <div class="absolute bottom-12 left-1/2 -translate-x-1/2 flex gap-4 px-8 w-full justify-center">
              <TestimonialCard testimonial={props.testimonials![0]} delay="animate-delay-1000" />
              <Show when={props.testimonials![1]}>
                <div class="hidden xl:flex">
                  <TestimonialCard testimonial={props.testimonials![1]} delay="animate-delay-1200" />
                </div>
              </Show>
            </div>
          </Show>
        </section>
      </Show>
    </div>
  );
};
