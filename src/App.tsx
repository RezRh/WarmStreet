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
  // Client-side control
  status?: 'Loading' | 'Signup' | 'Login' | 'Unauthenticated' | 'Ready' | 'Error';
  
  // Rust ViewState
  state?: {
    type: 'Loading' | 'Unauthenticated' | 'Ready' | 'Error' | 'OnboardingLocation' | 'Authenticating';
    [key: string]: any;
  };

  // Shared Data
  feed_view?: 'map' | 'list';
  cases?: Case[];
  selected_case?: Case | null;
  is_refreshing?: boolean;
  error?: string | null;
  toast?: string | null;
  community_members: any[];
  is_loading_community: boolean;
  active_chat_member: any | null;
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
  community_members: [
    {
      id: "vet1",
      name: "Paws & Claws Veterinary Clinic",
      member_type: "Vet",
      description: "Excellence in small animal care with specialized surgical equipment and 24/7 emergency service.",
      location_name: "Ramesh Nagar, Delhi",
      phone: "+919876543210",
      image_url: "https://images.unsplash.com/photo-1584132967334-10e028bd69f7?w=800&q=80",
      lat: 28.6508,
      lon: 77.1352,
    },
    {
      id: "ngo1",
      name: "Friendicoes SECA",
      member_type: "NGO",
      description: "Oldest animal shelter in Delhi providing hospital care, outpatient clinic, and ambulance service for street animals.",
      location_name: "Defence Colony, Delhi",
      phone: "+911124320701",
      image_url: "https://images.unsplash.com/photo-1602491673980-73aad856d8cc?w=800&q=80",
      lat: 28.5724,
      lon: 77.2215,
    },
    {
      id: "vet2",
      name: "The Pet Hospital",
      member_type: "Vet",
      description: "Modern facility offering advanced diagnostics, grooming, and specialized orthopedic surgery.",
      location_name: "Vasant Kunj, Delhi",
      phone: "+919988776655",
      image_url: "https://images.unsplash.com/photo-1628009368231-7bb7cfcb0def?w=800&q=80",
      lat: 28.5293,
      lon: 77.1517,
    },
    {
      id: "ngo2",
      name: "Wildlife SOS",
      member_type: "NGO",
      description: "Dedicated to protecting and conserving India's rich biodiversity. Specialises in elephant and bear rescue.",
      location_name: "South Ext, Delhi",
      phone: "+919871963535",
      image_url: "https://images.unsplash.com/photo-1606103920295-9a091573f160?w=800&q=80",
      lat: 28.5714,
      lon: 77.2185,
    },
    {
      id: "vet5",
      name: "Gurgaon Pet Hospital",
      member_type: "Vet",
      description: "24-hour emergency hospital with advanced diagnostics and surgery in the heart of Gurgaon.",
      location_name: "Sector 45, Gurgaon",
      phone: "+911244001234",
      image_url: "https://images.unsplash.com/photo-1532938911079-1b06ac7ceec7?w=800&q=80",
      lat: 28.4595,
      lon: 77.0266,
    },
    {
      id: "ngo5",
      name: "Umeed For Animals Foundation",
      member_type: "NGO",
      description: "Rescue and rehabilitation center for sick and injured street animals in Gurgaon.",
      location_name: "DLF Phase 1, Gurgaon",
      phone: "+919999956541",
      image_url: "https://images.unsplash.com/photo-1541591419107-ee82f7f9b7cb?w=800&q=80",
      lat: 28.4722,
      lon: 77.0863,
    },
    {
      id: "vet6",
      name: "Noida Animal Clinic",
      member_type: "Vet",
      description: "Specialized avian and exotic pet care alongside domestic cat and dog medicine.",
      location_name: "Sector 18, Noida",
      phone: "+911204123456",
      image_url: "https://images.unsplash.com/photo-1583337130417-3346a1be7dee?w=800&q=80",
      lat: 28.5708,
      lon: 77.3271,
    },
    {
      id: "ngo6",
      name: "Compassion for Animals",
      member_type: "NGO",
      description: "Promoting kindness through education and direct rescue operations in East Delhi and Noida.",
      location_name: "Sector 62, Noida",
      phone: "+919811223344",
      image_url: "https://images.unsplash.com/photo-1450778869180-41d0601e046e?w=800&q=80",
      lat: 28.6258,
      lon: 77.3732,
    },
    {
      id: "vet3",
      name: "Dr. Kapoor's Pet Clinic",
      member_type: "Vet",
      description: "Personalized care for domestic pets and birds. Specializing in vaccinations and preventive medicine.",
      location_name: "Janakpuri, Delhi",
      phone: "+919810012345",
      image_url: "https://images.unsplash.com/photo-1599443015574-be5fe8a05783?w=800&q=80",
      lat: 28.6219,
      lon: 77.0878,
    },
    {
      id: "ngo3",
      name: "Red Paws Rescue",
      member_type: "NGO",
      description: "Focused on managing the stray dog population through sterilisation and finding forever homes through adoption.",
      location_name: "Sainik Farms, Delhi",
      phone: "+919958156621",
      image_url: "https://images.unsplash.com/photo-1591768793355-74dcaaf41850?w=800&q=80",
      lat: 28.5135,
      lon: 77.2144,
    },
    {
      id: "vet4",
      name: "Max Vets Specialized Hospital",
      member_type: "Vet",
      description: "Multi-specialty hospital with radiology, cardiology, and a 24-hour trauma center for critical cases.",
      location_name: "East of Kailash, Delhi",
      phone: "+911140503070",
      image_url: "https://images.unsplash.com/photo-1527672829624-f3a142faafec?w=800&q=80",
      lat: 28.5588,
      lon: 77.2458,
    },
    {
      id: "ngo4",
      name: "People for Animals (PFA)",
      member_type: "NGO",
      description: "India's largest animal welfare organization. Operates shelters, ambulances, and runs awareness campaigns.",
      location_name: "Connaught Place, Delhi",
      phone: "+911123357088",
      image_url: "https://images.unsplash.com/photo-1544568100-847a948585b9?w=800&q=80",
      lat: 28.6328,
      lon: 77.2197,
    },
    {
      id: "vet7",
      name: "Pet Care Clinic",
      member_type: "Vet",
      description: "Comprehensive veterinary services including pathology, surgery, and grooming for all small animals.",
      location_name: "Sector 10, Dwarka",
      phone: "+919911223344",
      image_url: "https://images.unsplash.com/photo-1576201836106-db1758fd1c97?w=800&q=80",
      lat: 28.5823,
      lon: 77.0500,
    }
  ],
  is_loading_community: false,
  active_chat_member: null,
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
        console.log('✅ ViewModel update from Rust:', event.payload);
        // Merge with existing state to preserve 'status' if needed, 
        // but prefer Rust's state for logic
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
      console.warn('⚠️ Browser Mock Dispatch:', event, payload);
      
      // Mock Direct Chat for Browser Testing
      if (event === 'MessageMemberRequested') {
        const member = initial_view_model.community_members.find(m => m.id === payload.member_id);
        set_view_model(prev => ({ 
          ...prev, 
          active_chat_member: member 
        }));
      }
      
      if (event === 'ChatClosed') {
        set_view_model(prev => ({ 
          ...prev, 
          active_chat_member: null 
        }));
      }
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

  const isAppReady = () => {
    const vm = view_model();
    return vm.status === 'Ready' || (vm.state && vm.state.type === 'Ready');
  };

  return (
    <>
      <Show when={needs_verification()}>
        <VerificationPage 
          onComplete={handleVerificationComplete} 
          onSignOut={handleSignOut} 
        />
      </Show>

      {isAppReady() ? (
        <HomePage 
          cases={view_model().status === 'Ready' ? view_model().cases! : view_model().state?.list_items || []}
          communityMembers={view_model().community_members}
          isLoadingCommunity={view_model().is_loading_community}
          activeChatMember={view_model().active_chat_member}
          onReport={() => dispatch('ReportSpotted')}
          onRefresh={() => console.log('Refresh clicked')}
          onCaseSelect={(id) => dispatch('CaseSelected', { case_id: id })}
          onMessageMember={(id) => dispatch('MessageMemberRequested', { member_id: id })}
          onCloseChat={() => dispatch('ChatClosed')}
          onSendMessage={(text) => console.log('Message sent:', text)}
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
