import { createSignal, onMount, Show } from 'solid-js';
import { listen } from '@tauri-apps/api/event';
import { SignInPage, Testimonial } from './components/ui/sign-in';
import { VerificationPage } from './components/ui/verification';
import { MfaVerificationPage } from './components/ui/mfa-verification';
import { HomePage } from './components/ui/home';
import { account } from './lib/appwrite';
import { ID, OAuthProvider } from 'appwrite';
import './App.css';

import { ViewModel } from './lib/types';
import { initializeMessaging, cleanupMessaging } from './lib/messaging';

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
  status: 'Unauthenticated',
  feed_view: 'map',
  cases: [],
  map_pins: [],
  selected_case: null,
  is_refreshing: false,
  error: null,
  toast: null,
  community_members: [],
  is_loading_community: false,
  active_chat_member: null,
  profile: null,
  conversations: []
};

function App() {
  const [view_model, set_view_model] = createSignal<ViewModel>(initial_view_model);
  const [auth_mode, set_auth_mode] = createSignal<'signin' | 'signup'>('signin');
  const [needs_verification, set_needs_verification] = createSignal(false);
  const [mfa_challenge_id, set_mfa_challenge_id] = createSignal<string | null>(null);
  const [currentUser, setCurrentUser] = createSignal<any>(null);

  onMount(async () => {
    console.log('🏠 WarmStreet App mounting...');

    // Check if we are in Tauri
    const isTauri = !!(window as any).__TAURI_INTERNALS__;
    console.log('Is Tauri environment:', isTauri);

    // Initial session check
    try {
      const user = await account.get();
      console.log('✅ Appwrite session found:', user);
      setCurrentUser(user);

      const prefs = user.prefs as any;
      if (!prefs || !prefs.user_type) {
        console.log('⚠️ Profile incomplete, needs verification');
        set_needs_verification(true);
      } else {
        // Initialize E2E encrypted messaging
        initializeMessaging()
          .then(() => console.log('✅ Messaging initialized'))
          .catch(err => console.error('❌ Messaging init failed:', err));

        // Set status to Ready
        set_view_model(prev => ({ ...prev, status: 'Ready' }));
      }
    } catch (err) {
      console.log('ℹ️ No active session');
    }

    if (isTauri) {
      // Listen for native map events
      const unlistenMapPin = listen('map-pin-tapped', (event: any) => {
        console.log('📍 Map pin tapped:', event.payload);
        // Handle map pin tap - select the case
        const caseId = event.payload;
        set_view_model(prev => ({
          ...prev,
          selected_case: prev.cases.find(c => c.id === caseId) || null,
        }));
      });

      const unlistenMapReady = listen('map-ready', () => {
        console.log('🗺️ Native map ready');
      });

      return () => {
        unlistenMapPin.then(f => f());
        unlistenMapReady.then(f => f());
      };
    }
  });

  const handleSignIn = async (e: Event) => {
    e.preventDefault();
    const form = e.target as HTMLFormElement;
    const formData = new FormData(form);
    const { email, password } = Object.fromEntries(formData.entries()) as any;
    
    try {
      const session = await account.createEmailPasswordSession(email, password);
      
      // Check for MFA requirement
      if ((session as any).mfa) {
        console.log('🛡️ MFA required');
        const challenge = await (account as any).createMfaChallenge('totp'); // Default to TOTP, can fallback to email
        set_mfa_challenge_id(challenge.$id);
        set_view_model(prev => ({ ...prev, status: 'MfaVerification' }));
        return;
      }

      const user = await account.get();
      console.log('🔐 Sign In success');
      
      const prefs = user.prefs as any;
      if (!prefs || !prefs.user_type) {
        set_needs_verification(true);
        return;
      }

      set_view_model(prev => ({ ...prev, status: 'Ready' }));
    } catch (err: any) {
      console.error('❌ Sign In failed:', err);
      set_view_model(prev => ({ ...prev, error: err.message }));
    }
  };

  const handleVerifyMfa = async (code: string) => {
    const challenge_id = mfa_challenge_id();
    if (!challenge_id) return;

    try {
      await (account as any).updateMfaChallenge(challenge_id, code);
      console.log('✅ MFA verified');
      
      const user = await account.get();
      
      const prefs = user.prefs as any;
      if (!prefs || !prefs.user_type) {
        set_needs_verification(true);
        set_mfa_challenge_id(null);
        return;
      }

      set_view_model(prev => ({ ...prev, status: 'Ready' }));
      set_mfa_challenge_id(null);
    } catch (err: any) {
      console.error('❌ MFA verification failed:', err);
      throw err;
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
      await account.createEmailPasswordSession(email, password);
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

      set_view_model(prev => ({ ...prev, status: 'Ready' }));
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
  };

  const handleCreateAccount = () => {
    console.log('➕ Create Account clicked');
    set_auth_mode('signup');
  };

  const handleContinueAsGuest = () => {
    console.log('👤 Continue as Guest clicked');
    set_view_model(prev => ({ ...prev, status: 'Ready' }));
  };

  const handleVerificationComplete = async (metadata: any) => {
    console.log('✅ Verification complete:', metadata);
    try {
      set_needs_verification(false);
      set_view_model(prev => ({ ...prev, status: 'Ready' }));
    } catch (err) {
      console.error('❌ Failed to finalize login after verification:', err);
    }
  };

  const handleSignOut = async () => {
    console.log('🚪 Sign Out clicked');
    try {
      // Clean up messaging encryption keys
      await cleanupMessaging();

      await account.deleteSession('current');
      set_view_model(prev => ({ ...prev, status: 'Unauthenticated' }));
      set_auth_mode('signin');
    } catch (err: any) {
      console.error('❌ Sign Out failed:', err);
    }
  };

  const isAppReady = () => {
    const vm = view_model();
    return vm.status === 'Ready';
  };

  return (
    <>
      <Show when={needs_verification()}>
        <VerificationPage 
          onComplete={handleVerificationComplete} 
          onSignOut={handleSignOut} 
        />
      </Show>

      <Show when={view_model().status === 'MfaVerification'}>
        <MfaVerificationPage
          onVerify={handleVerifyMfa}
          onCancel={() => {
            set_mfa_challenge_id(null);
            // Logout - clean up messaging and session
            cleanupMessaging();
            set_view_model(prev => ({ ...prev, status: 'Unauthenticated' }));
          }}
        />
      </Show>

      {isAppReady() ? (
        <HomePage
          user={currentUser()}
          cases={view_model().cases}
          communityMembers={view_model().community_members}
          isLoadingCommunity={view_model().is_loading_community}
          activeChatMember={view_model().active_chat_member}
          profile={view_model().profile}
          onReport={() => console.log('📸 Report spotted clicked')}
          onRefresh={() => console.log('🔄 Refresh clicked')}
          onCaseSelect={(id) => {
            console.log('📍 Case selected:', id);
            set_view_model(prev => ({
              ...prev,
              selected_case: prev.cases.find(c => c.id === id) || null,
            }));
          }}
          onMessageMember={(id) => {
            console.log('💬 Message member:', id);
            const member = view_model().community_members.find(m => m.id === id);
            set_view_model(prev => ({
              ...prev,
              active_chat_member: member || null,
            }));
          }}
          onCloseChat={() => {
            console.log('❌ Chat closed');
            set_view_model(prev => ({ ...prev, active_chat_member: null }));
          }}
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
