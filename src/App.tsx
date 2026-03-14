import { createSignal, onMount, Show } from 'solid-js';
import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import { SignInPage, Testimonial } from './components/ui/sign-in';
import { VerificationPage } from './components/ui/verification';
import { HomePage } from './components/ui/home';
import { account } from './lib/appwrite';
import { ID, OAuthProvider } from 'appwrite';
import './App.css';

// ViewModel from Crux Rust core
interface ViewModel {
  status: 'Loading' | 'Signup' | 'Login' | 'Unauthenticated' | 'Ready' | 'Error';
  feed_view: 'map' | 'list';
  cases: Case[];
  selected_case: Case | null;
  is_refreshing: boolean;
  error: string | null;
  toast: string | null;
  community_members: any[];
  is_loading_community: boolean;
}

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

const sampleTestimonials: Testimonial[] = [
  {
    avatarSrc: "https://images.unsplash.com/photo-1494790108377-be9c29b29330?w=150&h=150&fit=crop",
    name: "Sarah Chen",
    handle: "@sarahrescue",
    text: "This platform helped me save 3 injured animals last month. The coordination is incredible!"
  },
  {
    avatarSrc: "https://images.unsplash.com/photo-1507003211169-0a1dd7228f2d?w=150&h=150&fit=crop",
    name: "Marcus Johnson",
    handle: "@marcusvet",
    text: "As a veterinarian, I can see how this reduces response time. Animals get help faster."
  },
  {
    avatarSrc: "https://images.unsplash.com/photo-1438761681033-6461ffad8d80?w=150&h=150&fit=crop",
    name: "David Martinez",
    handle: "@davidvolunteer",
    text: "The real-time updates mean I never miss a rescue opportunity. Game changer!"
  },
];

const initial_view_model: ViewModel = {
  status: 'Signup',
  feed_view: 'map',
  cases: [
    {
      id: '1',
      description: 'Dog scratching ear severely',
      status: 'IN PROGRESS',
      severity: 'Moderate',
      type: 'Dog',
      age: 'Adult',
      breed: 'Mixed/Unknown',
      imageUrl: 'https://images.unsplash.com/photo-1541233349642-6e425fe6190e?w=400&q=80',
      date: '12/4/2025'
    },
    {
      id: '2',
      description: 'Cat with wounded leg',
      status: 'SUBMITTED',
      severity: 'High',
      type: 'Cat',
      age: 'Young',
      breed: 'Persian',
      imageUrl: 'https://images.unsplash.com/photo-1543852786-1cf6624b9987?w=400&q=80',
      date: '12/4/2025'
    }
  ],
  selected_case: null,
  is_refreshing: false,
  error: null,
  toast: null,
  community_members: [],
  is_loading_community: false,
};

function App() {
  const [view_model, set_view_model] = createSignal<ViewModel>(initial_view_model);
  const [auth_mode, set_auth_mode] = createSignal<'signin' | 'signup'>('signin');
  const [needs_verification, set_needs_verification] = createSignal(false);

  onMount(async () => {
    console.log('🏠 WarmStreet App mounting...');
    
    // Check for existing Appwrite session
    try {
      const user = await account.get();
      console.log('✅ Appwrite session found:', user);
      
      const prefs = user.prefs as any;
      
      if (!prefs || !prefs.user_type) {
        console.log('⚠️ Profile incomplete, needs verification');
        set_needs_verification(true);
        return;
      }

      // Get session for JWT equivalent
      const session = await account.getSession('current');
      
      dispatch('LoginCompleted', { 
        jwt: session.$id, 
        user_id: user.$id,
        metadata: prefs
      });
    } catch (err) {
      console.log('ℹ️ No active session');
    }

    // Check if we are in Tauri
    const isTauri = !!(window as any).__TAURI_INTERNALS__;
    console.log('Is Tauri environment:', isTauri);

    if (isTauri) {
      const unlisten = listen('crux-update', (event: any) => {
        console.log('✅ ViewModel update:', event.payload);
        set_view_model(event.payload);
      });

      invoke('handle_event', { event: 'AppStarted' })
        .then(() => console.log('✅ AppStarted dispatched'))
        .catch(err => console.error('❌ Dispatch failed:', err));

      return () => unlisten.then(f => f());
    } else {
      console.warn('⚠️ Running in browser: Tauri features disabled');
    }
  });

  const dispatch = (event: string, payload?: any) => {
    console.log('📤 Dispatch:', event, payload);
    
    // Check if we are in Tauri
    const isTauri = !!(window as any).__TAURI_INTERNALS__;
    
    if (isTauri) {
      invoke('handle_event', { event, payload });
    } else {
      console.warn('⚠️ Browser Mock Dispatch:', event);
      // Simulate state transitions for browser testing if needed
      if (event === 'OnboardingComplete') {
        set_view_model(prev => ({ ...prev, status: 'Ready' }));
      }
    }
  };

  const handleSignIn = async (e: Event) => {
    e.preventDefault();
    const form = e.target as HTMLFormElement;
    const formData = new FormData(form);
    const { email, password } = Object.fromEntries(formData.entries()) as any;
    
    try {
      const session = await account.createEmailPasswordSession(email, password);
      const user = await account.get();
      console.log('🔐 Sign In success');
      
      const prefs = user.prefs as any;
      if (!prefs || !prefs.user_type) {
        set_needs_verification(true);
        return;
      }

      dispatch('LoginCompleted', { 
        jwt: session.$id, 
        user_id: user.$id,
        metadata: prefs
      });
    } catch (err: any) {
      console.error('❌ Sign In failed:', err);
      set_view_model(prev => ({ ...prev, error: err.message }));
    }
  };

  const handleSignup = async (e: Event) => {
    e.preventDefault();
    const form = e.target as HTMLFormElement;
    const formData = new FormData(form);
    const { email, password, name } = Object.fromEntries(formData.entries()) as any;
    
    try {
      const userType = formData.get('user_type') as string;
      await account.create(ID.unique(), email, password, name);
      const session = await account.createEmailPasswordSession(email, password);
      const user = await account.get();
      console.log('📝 Sign Up success as', userType);
      
      const metadata: any = { user_type: userType };
      if (userType === 'ngo') {
        metadata.org_name = formData.get('org_name');
        metadata.reg_number = formData.get('reg_number');
        metadata.phone = formData.get('phone');
      } else if (userType === 'vet') {
        metadata.clinic_name = formData.get('clinic_name');
        metadata.license_number = formData.get('license_number');
        metadata.phone = formData.get('phone');
      }

      // Persist to Appwrite prefs so it's available after social logins/re-logins
      await account.updatePrefs(metadata);

      dispatch('LoginCompleted', { 
        jwt: session.$id, 
        user_id: user.$id,
        metadata
      });
    } catch (err: any) {
      console.error('❌ Sign Up failed:', err);
      set_view_model(prev => ({ ...prev, error: err.message }));
    }
  };

  const handleGoogleSignIn = () => {
    console.log('🔵 Google Sign In clicked');
    account.createOAuth2Session(OAuthProvider.Google, 'http://localhost:5173', 'http://localhost:5173/login');
  };

  const handleAppleSignIn = () => {
    console.log('🍎 Apple Sign In clicked');
    account.createOAuth2Session(OAuthProvider.Apple, 'http://localhost:5173', 'http://localhost:5173/login');
  };

  const handleResetPassword = () => {
    console.log('🔑 Reset Password clicked');
    dispatch('ResetPasswordRequested');
  };

  const handleCreateAccount = () => {
    console.log('➕ Create Account clicked');
    set_auth_mode('signup');
  };

  const handleContinueAsGuest = () => {
    console.log('👤 Continue as Guest clicked');
    dispatch('OnboardingComplete');
  };

  const handleVerificationComplete = async (metadata: any) => {
    console.log('✅ Verification complete:', metadata);
    try {
      const user = await account.get();
      const session = await account.getSession('current');
      set_needs_verification(false);
      dispatch('LoginCompleted', { 
        jwt: session.$id, 
        user_id: user.$id,
        metadata
      });
    } catch (err) {
      console.error('❌ Failed to finalize login after verification:', err);
    }
  };

  const handleSignOut = async () => {
    console.log('🚪 Sign Out clicked');
    try {
      await account.deleteSession('current');
      set_view_model(prev => ({ ...prev, status: 'Signup' })); // or initial status
      dispatch('LogoutRequested');
      set_auth_mode('signin');
    } catch (err: any) {
      console.error('❌ Sign Out failed:', err);
    }
  };

  return (
    <>
      <Show when={needs_verification()}>
        <VerificationPage 
          onComplete={handleVerificationComplete} 
          onSignOut={handleSignOut} 
        />
      </Show>

      {view_model().status === 'Ready' ? (
        <HomePage 
          cases={view_model().cases as any}
          communityMembers={view_model().community_members}
          isLoadingCommunity={view_model().is_loading_community}
          onReport={() => dispatch('ReportSpotted')}
          onRefresh={() => console.log('Refresh clicked')}
          onCaseSelect={(id) => dispatch('CaseSelected', { case_id: id })}
          onMessageMember={(id) => dispatch('MessageMemberRequested', { member_id: id })}
          onSignOut={handleSignOut}
        />
      ) : (
        // Auth View (Sign In / Sign Up)
        <SignInPage
          heroImageSrc="https://images.unsplash.com/photo-1548199973-03cce0bbc87b?w=1920&q=80"
          testimonials={sampleTestimonials}
          mode={auth_mode()}
          onSignIn={handleSignIn}
          onSignup={handleSignup}
          onGoogleSignIn={handleGoogleSignIn}
          onAppleSignIn={handleAppleSignIn}
          onContinueAsGuest={handleContinueAsGuest}
          onCreateAccount={handleCreateAccount}
          onResetPassword={handleResetPassword}
          title={auth_mode() === 'signup' ? 'Create Account' : 'Welcome Back'}
          description={auth_mode() === 'signup' ? 'Join the fastest animal rescue network' : 'Sign in to continue saving animals'}
        />
      )}
    </>
  );
}

export default App;
