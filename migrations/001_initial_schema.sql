-- 1. Enable PostGIS extension
CREATE EXTENSION IF NOT EXISTS postgis;

-- 2. Create tables

-- public.user_profile
CREATE TABLE public.user_profile (
    user_id TEXT PRIMARY KEY,
    role TEXT NOT NULL DEFAULT 'citizen' CHECK (role IN ('citizen','volunteer','ngo','vet','admin')),
    verification_status TEXT NOT NULL DEFAULT 'unverified',
    trust_score INT NOT NULL DEFAULT 0,
    area_center GEOGRAPHY(POINT, 4326),
    area_radius_m INT NOT NULL DEFAULT 5000 CHECK (area_radius_m IN (2000,5000,10000,20000,25000)),
    last_location GEOGRAPHY(POINT, 4326),
    last_active TIMESTAMPTZ DEFAULT now(),
    fcm_token TEXT,
    created_at TIMESTAMPTZ DEFAULT now(),
    updated_at TIMESTAMPTZ DEFAULT now(),
    
    -- GIST indexes
    CONSTRAINT valid_area_center CHECK (area_center IS NULL OR (ST_X(area_center::geometry) BETWEEN -180 AND 180 AND ST_Y(area_center::geometry) BETWEEN -90 AND 90))
);

CREATE INDEX idx_user_profile_area_center ON public.user_profile USING GIST (area_center);
CREATE INDEX idx_user_profile_last_location ON public.user_profile USING GIST (last_location);

-- public.rescue_cases
CREATE TABLE public.rescue_cases (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    reporter_id TEXT NOT NULL REFERENCES public.user_profile(user_id) ON DELETE CASCADE,
    assigned_rescuer_id TEXT REFERENCES public.user_profile(user_id) ON DELETE SET NULL,
    location GEOGRAPHY(POINT, 4326) NOT NULL,
    description TEXT,
    landmark_hint TEXT,
    wound_severity INT CHECK (wound_severity BETWEEN 1 AND 10),
    ai_confidence REAL,
    yolo_bbox JSONB,
    gemini_diagnosis JSONB,
    photo_object_key TEXT,
    crop_object_key TEXT,
    status TEXT NOT NULL DEFAULT 'pending' CHECK (status IN ('pending','claimed','en_route','arrived','resolved','cancelled','unreachable')),
    visibility TEXT NOT NULL DEFAULT 'visible',
    resolved_at TIMESTAMPTZ,
    created_at TIMESTAMPTZ DEFAULT now(),
    updated_at TIMESTAMPTZ DEFAULT now()
);

CREATE INDEX idx_rescue_cases_location ON public.rescue_cases USING GIST (location);
CREATE INDEX idx_rescue_cases_status ON public.rescue_cases (status) WHERE status != 'resolved';
CREATE INDEX idx_rescue_cases_visible ON public.rescue_cases (visibility) WHERE visibility = 'visible';

-- public.idempotency_log
CREATE TABLE public.idempotency_log (
    user_id TEXT NOT NULL,
    endpoint TEXT NOT NULL,
    key TEXT NOT NULL,
    response JSONB,
    created_at TIMESTAMPTZ DEFAULT now(),
    PRIMARY KEY (user_id, endpoint, key)
);

-- public.case_notifications_sent
CREATE TABLE public.case_notifications_sent (
    case_id UUID NOT NULL,
    user_id TEXT NOT NULL,
    kind TEXT NOT NULL,
    sent_at TIMESTAMPTZ DEFAULT now(),
    PRIMARY KEY (case_id, user_id, kind)
);

-- 3. Create helper function for Neon Auth
CREATE OR REPLACE FUNCTION app_auth.user_id() RETURNS TEXT AS $$
  SELECT current_setting('app.user_id', true);
$$ LANGUAGE SQL STABLE;

-- 4. Create functions

-- Atomic claim
CREATE OR REPLACE FUNCTION public.claim_rescue_case(p_case_id UUID, p_rescuer_id TEXT)
RETURNS BOOLEAN AS $$
BEGIN
  UPDATE public.rescue_cases
  SET assigned_rescuer_id = p_rescuer_id, status = 'claimed', updated_at = now()
  WHERE id = p_case_id
    AND status = 'pending'
    AND assigned_rescuer_id IS NULL;
  
  RETURN FOUND;
END;
$$ LANGUAGE plpgsql SECURITY DEFINER;

-- Validated status transition
CREATE OR REPLACE FUNCTION public.transition_case(p_case_id UUID, p_actor_id TEXT, p_next TEXT)
RETURNS BOOLEAN AS $$
DECLARE
  current_status TEXT;
BEGIN
  SELECT status INTO current_status
  FROM public.rescue_cases WHERE id = p_case_id;

  IF NOT FOUND THEN RETURN FALSE; END IF;

  IF (current_status, p_next) IN (
    ('pending', 'claimed'),
    ('claimed', 'en_route'),
    ('en_route', 'arrived'),
    ('arrived', 'resolved'),
    ('claimed', 'cancelled'),
    ('pending', 'cancelled'),
    ('claimed', 'unreachable'),
    ('en_route', 'unreachable')
  ) AND (
    p_actor_id = (SELECT reporter_id FROM public.rescue_cases WHERE id = p_case_id)
    OR p_actor_id = (SELECT assigned_rescuer_id FROM public.rescue_cases WHERE id = p_case_id)
    OR EXISTS (SELECT 1 FROM public.user_profile WHERE user_id = p_actor_id AND role = 'admin')
  ) THEN
    UPDATE public.rescue_cases
    SET status = p_next,
        updated_at = now(),
        resolved_at = CASE WHEN p_next = 'resolved' THEN now() ELSE resolved_at END
    WHERE id = p_case_id;
    RETURN TRUE;
  END IF;

  RETURN FALSE;
END;
$$ LANGUAGE plpgsql SECURITY DEFINER;

-- Nearby volunteers finder
CREATE OR REPLACE FUNCTION public.get_nearby_volunteers(case_location GEOGRAPHY, radius_meters INT DEFAULT 25000)
RETURNS TABLE(
  user_id TEXT,
  role TEXT,
  distance_m DOUBLE PRECISION
) AS $$
  SELECT 
    up.user_id,
    up.role,
    ST_Distance(up.last_location, case_location) AS distance_m
  FROM public.user_profile up
  WHERE up.last_location IS NOT NULL
    AND up.role IN ('volunteer','ngo','vet')
    AND ST_DWithin(up.last_location, case_location, radius_meters)
  ORDER BY distance_m;
$$ LANGUAGE sql STABLE;

-- 5. Enable RLS and add policies

ALTER TABLE public.rescue_cases ENABLE ROW LEVEL SECURITY;

-- Reporter can read their own cases
CREATE POLICY reporter_read_own ON public.rescue_cases FOR SELECT
  USING (reporter_id = app_auth.user_id());

-- Assigned rescuer can read their cases
CREATE POLICY rescuer_read_assigned ON public.rescue_cases FOR SELECT
  USING (assigned_rescuer_id = app_auth.user_id());

-- Volunteers see nearby visible pending cases within their area_radius_m
CREATE POLICY read_nearby_visible ON public.rescue_cases FOR SELECT
  USING (
    visibility = 'visible'
    AND status = 'pending'
    AND EXISTS (
      SELECT 1 FROM public.user_profile up
      WHERE up.user_id = app_auth.user_id()
        AND up.area_center IS NOT NULL
        AND ST_DWithin(rescue_cases.location, up.area_center, up.area_radius_m)
    )
  );
