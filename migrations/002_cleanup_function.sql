-- Migration 002: Add cleanup function for media keys
-- Ensures photo/crop object keys are nullified after case resolution

CREATE OR REPLACE FUNCTION public.cleanup_media_keys(p_case_id UUID)
RETURNS VOID AS $$
BEGIN
  UPDATE public.rescue_cases
  SET photo_object_key = NULL, 
      crop_object_key = NULL,
      updated_at = now()
  WHERE id = p_case_id;
END;
$$ LANGUAGE plpgsql SECURITY DEFINER;

COMMENT ON FUNCTION public.cleanup_media_keys IS 'Nullify media object keys after case resolution to prepare for Tigris cleanup';
