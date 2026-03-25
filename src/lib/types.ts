export interface Case {
  id: string;
  description: string;
  status: string;
  severity: number;
  type: string;
  age?: string;
  breed?: string;
  image_url?: string;
  created_at: string;
}

export interface CommunityMember {
  id: string;
  name: string;
  member_type: 'Individual' | 'Vet' | 'NGO' | string;
  description: string;
  location_name: string;
  phone: string;
  image_url: string;
  lat: number;
  lon: number;
  karma: number;
  last_active: string;
}

export interface UserProfile {
  name: string;
  email: string;
  phone?: string;
  member_type: string;
  karma: number;
  rescues: number;
  verification_level: string;
}

export interface MapPin {
  case_id: string;
  lat: number;
  lon: number;
  severity: number;
  status: string;
}

export interface ViewModel {
  status: string;
  feed_view: 'map' | 'list';
  cases: Case[];
  map_pins: MapPin[];
  selected_case: Case | null;
  is_refreshing: boolean;
  error: string | null;
  toast: string | null;
  profile: UserProfile | null;
  community_members: CommunityMember[];
  is_loading_community: boolean;
  active_chat_member: CommunityMember | null;
}
