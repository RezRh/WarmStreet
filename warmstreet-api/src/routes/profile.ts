import { Hono } from 'hono';
import { z } from 'zod';
import { zValidator } from '@hono/zod-validator';
import { HonoEnv } from '../types';

const app = new Hono<HonoEnv>();

// 1. POST /bootstrap
app.post('/bootstrap', async (c) => {
  const user = c.get('user');
  const db = c.get('db');
  if (!user || !db) return c.json({ error: 'Internal Server Error' }, 500);

  // Upsert user_profile
  // Sets role = 'citizen', verified = 'unverified', trust = 0
  const query = `
    INSERT INTO public.user_profile (user_id, role, verification_status, trust_score)
    VALUES ($1, 'citizen', 'unverified', 0)
    ON CONFLICT (user_id) DO NOTHING
    RETURNING user_id
  `;

  const res = await db.query(query, [user.user_id]);
  const bootstrapped = res.rows.length > 0;

  return c.json({ success: true, bootstrapped });
});

// 2. PATCH /area
const areaSchema = z.object({
  lat: z.number().min(-90).max(90),
  lng: z.number().min(-180).max(180),
  radius_m: z.union([
    z.literal(2000),
    z.literal(5000),
    z.literal(10000),
    z.literal(20000),
    z.literal(25000)
  ]),
});

app.patch('/area', zValidator('json', areaSchema), async (c) => {
  const { lat, lng, radius_m } = c.req.valid('json');
  const user = c.get('user');
  const db = c.get('db');
  if (!user || !db) return c.json({ error: 'Internal Server Error' }, 500);

  const query = `
    UPDATE public.user_profile
    SET 
      area_center = ST_SetSRID(ST_MakePoint($1, $2), 4326),
      area_radius_m = $3,
      updated_at = now()
    WHERE user_id = $4
  `;

  await db.query(query, [lng, lat, radius_m, user.user_id]);

  return c.json({ success: true });
});

// 3. POST /fcm
const fcmSchema = z.object({
  token: z.string()
});

app.post('/fcm', zValidator('json', fcmSchema), async (c) => {
  const { token } = c.req.valid('json');
  const user = c.get('user');
  const db = c.get('db');
  if (!user || !db) return c.json({ error: 'Internal Server Error' }, 500);

  await db.query(
    `UPDATE public.user_profile SET fcm_token = $1, updated_at = now() WHERE user_id = $2`,
    [token, user.user_id]
  );

  return c.json({ success: true });
});

export default app;
