// index.ts - Cloudflare Worker for S3/Tigris Presigned URL Generation
// Production-ready implementation with full security controls

import { z } from 'zod';

const CONFIG = {
    UPLOAD_URL_EXPIRY_SECONDS: 600,
    DOWNLOAD_URL_EXPIRY_SECONDS: 900,
    MAX_UPLOAD_SIZE_BYTES: 10 * 1024 * 1024,
    MAX_KINDS_PER_REQUEST: 4,
    RATE_LIMIT_WINDOW_SECONDS: 60,
    RATE_LIMIT_MAX_REQUESTS: 100,
    JWT_ALGORITHM: 'HS256',
    S3_SERVICE: 's3',
    AWS_SIGNATURE_VERSION: 'AWS4-HMAC-SHA256',
} as const;

const ALLOWED_KINDS = ['original', 'thumbnail', 'processed', 'crop'] as const;
type ImageKind = typeof ALLOWED_KINDS[number];

const ALLOWED_FORMATS: Record<string, string> = {
    webp: 'image/webp',
    jpg: 'image/jpeg',
    jpeg: 'image/jpeg',
    png: 'image/png',
};

const UUID_V4_REGEX = /^[0-9a-f]{8}-[0-9a-f]{4}-4[0-9a-f]{3}-[89ab][0-9a-f]{3}-[0-9a-f]{12}$/i;

interface Env {
    TIGRIS_BUCKET: string;
    AWS_ACCESS_KEY_ID: string;
    AWS_SECRET_ACCESS_KEY: string;
    AWS_REGION: string;
    AWS_ENDPOINT_URL_S3: string;
    JWT_SECRET: string;
    ALLOWED_ORIGINS: string;
    RATE_LIMIT: KVNamespace;
    INTERNAL_API_URL?: string;
    INTERNAL_API_KEY?: string;
}

interface JWTPayload {
    sub: string;
    exp: number;
    iat: number;
    roles?: string[];
}

interface AuthContext {
    userId: string;
    roles: string[];
    token: string;
}

interface PresignedUrlParams {
    method: 'GET' | 'PUT';
    bucket: string;
    key: string;
    expiresInSeconds: number;
    contentType?: string;
    contentLengthRange?: { min: number; max: number };
}

interface UploadUrlResponse {
    upload_url: string;
    object_key: string;
    expires_at: string;
    content_type: string;
    max_size_bytes: number;
    required_headers: Record<string, string>;
}

interface DownloadUrlResponse {
    url: string;
    expires_at: string;
    kind: ImageKind;
    format: string;
}

interface ErrorResponse {
    error: string;
    code: string;
    request_id: string;
}

interface RequestContext {
    requestId: string;
    startTime: number;
    method: string;
    path: string;
    ip: string;
}

const UploadRequestSchema = z.object({
    kinds: z
        .array(z.enum(ALLOWED_KINDS))
        .min(1, 'At least one kind is required')
        .max(CONFIG.MAX_KINDS_PER_REQUEST, `Maximum ${CONFIG.MAX_KINDS_PER_REQUEST} kinds allowed`),
    format: z.enum(['webp', 'jpg', 'jpeg', 'png'] as const).optional().default('webp'),
});


class AppError extends Error {
    public retryAfter?: number;
    constructor(
        public readonly code: string,
        public readonly message: string,
        public readonly statusCode: number,
        public readonly isOperational: boolean = true
    ) {
        super(message);
        this.name = 'AppError';
    }

    static badRequest(message: string, code: string = 'BAD_REQUEST'): AppError {
        return new AppError(code, message, 400);
    }

    static unauthorized(message: string = 'Authentication required'): AppError {
        return new AppError('UNAUTHORIZED', message, 401);
    }

    static forbidden(message: string = 'Access denied'): AppError {
        return new AppError('FORBIDDEN', message, 403);
    }

    static notFound(message: string = 'Resource not found'): AppError {
        return new AppError('NOT_FOUND', message, 404);
    }

    static rateLimited(retryAfter: number): AppError {
        const error = new AppError('RATE_LIMITED', 'Too many requests', 429);
        error.retryAfter = retryAfter;
        return error;
    }

    static internal(message: string = 'Internal server error'): AppError {
        return new AppError('INTERNAL_ERROR', message, 500, false);
    }
}

function generateRequestId(): string {
    return crypto.randomUUID();
}

function createRequestContext(request: Request): RequestContext {
    const url = new URL(request.url);
    return {
        requestId: generateRequestId(),
        startTime: Date.now(),
        method: request.method,
        path: url.pathname,
        ip: request.headers.get('CF-Connecting-IP') ||
            request.headers.get('X-Forwarded-For')?.split(',')[0]?.trim() ||
            'unknown',
    };
}

function log(ctx: RequestContext, level: 'info' | 'warn' | 'error', event: string, data: Record<string, unknown> = {}): void {
    console.log(JSON.stringify({
        timestamp: new Date().toISOString(),
        level,
        request_id: ctx.requestId,
        event,
        method: ctx.method,
        path: ctx.path,
        ip: ctx.ip,
        duration_ms: Date.now() - ctx.startTime,
        ...data,
    }));
}

function validateEnv(env: Env): void {
    const required: (keyof Env)[] = [
        'TIGRIS_BUCKET',
        'AWS_ACCESS_KEY_ID',
        'AWS_SECRET_ACCESS_KEY',
        'AWS_ENDPOINT_URL_S3',
        'JWT_SECRET',
        'ALLOWED_ORIGINS',
    ];

    const missing = required.filter(key => !env[key]);
    if (missing.length > 0) {
        throw new Error('Missing required environment configuration');
    }

    if (env.JWT_SECRET.length < 32) {
        throw new Error('JWT_SECRET must be at least 32 characters');
    }
}

function validateCaseId(caseId: string): boolean {
    if (!caseId || typeof caseId !== 'string') {
        return false;
    }
    return UUID_V4_REGEX.test(caseId);
}

function validateKind(kind: string): kind is ImageKind {
    return ALLOWED_KINDS.includes(kind as ImageKind);
}

function validateFormat(format: string): boolean {
    return Object.hasOwn(ALLOWED_FORMATS, format);
}

function sanitizeForLogging(value: string, maxLength: number = 100): string {
    if (!value) return '';
    return value.slice(0, maxLength).replace(/[^\x20-\x7E]/g, '');
}

function getCorsHeaders(env: Env, origin: string | null): Record<string, string> {
    const allowedOrigins = env.ALLOWED_ORIGINS.split(',').map(o => o.trim()).filter(Boolean);

    let allowOrigin = '';
    if (origin && allowedOrigins.includes(origin)) {
        allowOrigin = origin;
    } else if (allowedOrigins.length > 0 && !origin) {
        allowOrigin = allowedOrigins[0] ?? '';
    }

    if (!allowOrigin) {
        return {};
    }

    return {
        'Access-Control-Allow-Origin': allowOrigin,
        'Access-Control-Allow-Methods': 'GET, POST, OPTIONS',
        'Access-Control-Allow-Headers': 'Content-Type, Authorization, X-Request-ID',
        'Access-Control-Expose-Headers': 'X-Request-ID, X-RateLimit-Remaining',
        'Access-Control-Max-Age': '86400',
        'Vary': 'Origin',
    };
}

function handlePreflight(corsHeaders: Record<string, string>): Response {
    if (Object.keys(corsHeaders).length === 0) {
        return new Response(null, { status: 403 });
    }
    return new Response(null, { status: 204, headers: corsHeaders });
}

function jsonResponse<T>(
    data: T,
    status: number,
    ctx: RequestContext,
    corsHeaders: Record<string, string>,
    extraHeaders: Record<string, string> = {}
): Response {
    return new Response(JSON.stringify(data), {
        status,
        headers: {
            'Content-Type': 'application/json',
            'X-Request-ID': ctx.requestId,
            'Cache-Control': 'no-store, no-cache, must-revalidate',
            ...corsHeaders,
            ...extraHeaders,
        },
    });
}

function errorResponse(
    error: AppError,
    ctx: RequestContext,
    corsHeaders: Record<string, string>
): Response {
    const body: ErrorResponse = {
        error: error.message,
        code: error.code,
        request_id: ctx.requestId,
    };

    const extraHeaders: Record<string, string> = {};
    if (error.retryAfter !== undefined) {
        extraHeaders['Retry-After'] = String(error.retryAfter);
    }

    return jsonResponse(body, error.statusCode, ctx, corsHeaders, extraHeaders);
}

interface RateLimitResult {
    allowed: boolean;
    remaining: number;
    resetAt: number;
}

async function checkRateLimit(
    env: Env,
    key: string,
    ctx: RequestContext
): Promise<RateLimitResult> {
    if (!env.RATE_LIMIT) {
        return { allowed: true, remaining: CONFIG.RATE_LIMIT_MAX_REQUESTS, resetAt: 0 };
    }

    const windowStart = Math.floor(Date.now() / 1000 / CONFIG.RATE_LIMIT_WINDOW_SECONDS);
    const rateLimitKey = `ratelimit:${key}:${windowStart}`;

    try {
        const current = await env.RATE_LIMIT.get(rateLimitKey);
        const count = current ? parseInt(current, 10) : 0;

        if (count >= CONFIG.RATE_LIMIT_MAX_REQUESTS) {
            const resetAt = (windowStart + 1) * CONFIG.RATE_LIMIT_WINDOW_SECONDS;
            return {
                allowed: false,
                remaining: 0,
                resetAt,
            };
        }

        await env.RATE_LIMIT.put(rateLimitKey, String(count + 1), {
            expirationTtl: CONFIG.RATE_LIMIT_WINDOW_SECONDS * 2,
        });

        return {
            allowed: true,
            remaining: CONFIG.RATE_LIMIT_MAX_REQUESTS - count - 1,
            resetAt: (windowStart + 1) * CONFIG.RATE_LIMIT_WINDOW_SECONDS,
        };
    } catch (error) {
        log(ctx, 'warn', 'rate_limit_error', { error: String(error) });
        return { allowed: true, remaining: CONFIG.RATE_LIMIT_MAX_REQUESTS, resetAt: 0 };
    }
}

async function verifyJWT(token: string, secret: string): Promise<JWTPayload> {
    const parts = token.split('.');
    if (parts.length !== 3) {
        throw new Error('Invalid token format');
    }

    const [headerB64, payloadB64, signatureB64] = parts;
    if (headerB64 === undefined || payloadB64 === undefined || signatureB64 === undefined) {
        throw new Error('Invalid token structure');
    }

    const headerJson = atob(headerB64.replace(/-/g, '+').replace(/_/g, '/'));
    const header = JSON.parse(headerJson) as { alg: string };
    if (header.alg !== CONFIG.JWT_ALGORITHM) {
        throw new Error('Unsupported algorithm');
    }

    const key = await crypto.subtle.importKey(
        'raw',
        new TextEncoder().encode(secret),
        { name: 'HMAC', hash: 'SHA-256' },
        false,
        ['verify']
    );

    const signatureInput = new TextEncoder().encode(`${headerB64}.${payloadB64}`);

    let signatureBytes: Uint8Array;
    try {
        const base64 = signatureB64.replace(/-/g, '+').replace(/_/g, '/');
        const padded = base64 + '='.repeat((4 - base64.length % 4) % 4);
        signatureBytes = Uint8Array.from(atob(padded), c => c.charCodeAt(0));
    } catch {
        throw new Error('Invalid signature encoding');
    }

    const valid = await crypto.subtle.verify('HMAC', key, signatureBytes, signatureInput);
    if (!valid) {
        throw new Error('Invalid signature');
    }

    const payloadJson = atob(payloadB64.replace(/-/g, '+').replace(/_/g, '/'));
    const payload = JSON.parse(payloadJson) as JWTPayload;

    const now = Math.floor(Date.now() / 1000);

    if (typeof payload.exp !== 'number' || payload.exp < now) {
        throw new Error('Token expired');
    }

    if (typeof payload.iat === 'number' && payload.iat > now + 60) {
        throw new Error('Token issued in the future');
    }

    if (!payload.sub || typeof payload.sub !== 'string') {
        throw new Error('Missing subject claim');
    }

    return payload;
}

async function authenticate(request: Request, env: Env, ctx: RequestContext): Promise<AuthContext> {
    const authHeader = request.headers.get('Authorization');

    if (!authHeader) {
        throw AppError.unauthorized('Missing Authorization header');
    }

    if (!authHeader.startsWith('Bearer ')) {
        throw AppError.unauthorized('Invalid Authorization header format');
    }

    const token = authHeader.slice(7).trim();
    if (!token) {
        throw AppError.unauthorized('Missing token');
    }

    try {
        const payload = await verifyJWT(token, env.JWT_SECRET);

        log(ctx, 'info', 'auth_success', { user_id: payload.sub });

        return {
            userId: payload.sub,
            roles: payload.roles || [],
            token,
        };
    } catch (error) {
        log(ctx, 'warn', 'auth_failed', { error: String(error) });
        throw AppError.unauthorized('Invalid or expired token');
    }
}

async function checkCaseAccess(
    caseId: string,
    auth: AuthContext,
    env: Env,
    ctx: RequestContext
): Promise<boolean> {
    if (auth.roles.includes('admin')) {
        return true;
    }

    if (!env.INTERNAL_API_URL || !env.INTERNAL_API_KEY) {
        log(ctx, 'warn', 'authz_skip', { reason: 'no_internal_api' });
        return true;
    }

    try {
        const response = await fetch(`${env.INTERNAL_API_URL}/internal/cases/${caseId}/access`, {
            method: 'POST',
            headers: {
                'Content-Type': 'application/json',
                'X-Internal-Key': env.INTERNAL_API_KEY,
                'X-Request-ID': ctx.requestId,
            },
            body: JSON.stringify({ userId: auth.userId }),
        });

        if (!response.ok) {
            log(ctx, 'info', 'authz_denied', { case_id: caseId, user_id: auth.userId, status: response.status });
            return false;
        }

        const result = await response.json() as { allowed: boolean };
        return result.allowed === true;
    } catch (error) {
        log(ctx, 'error', 'authz_error', { error: String(error) });
        return false;
    }
}

async function sha256(message: string | ArrayBuffer): Promise<ArrayBuffer> {
    const data = typeof message === 'string' ? new TextEncoder().encode(message) : message;
    return crypto.subtle.digest('SHA-256', data);
}

async function sha256Hex(message: string): Promise<string> {
    const hash = await sha256(message);
    return bufferToHex(hash);
}

function bufferToHex(buffer: ArrayBuffer): string {
    return Array.from(new Uint8Array(buffer))
        .map(b => b.toString(16).padStart(2, '0'))
        .join('');
}

async function hmacSign(key: CryptoKey, message: string): Promise<ArrayBuffer> {
    const data = new TextEncoder().encode(message);
    return crypto.subtle.sign('HMAC', key, data);
}

async function importHmacKey(keyData: ArrayBuffer | string): Promise<CryptoKey> {
    const data = typeof keyData === 'string' ? new TextEncoder().encode(keyData) : keyData;
    return crypto.subtle.importKey(
        'raw',
        data,
        { name: 'HMAC', hash: 'SHA-256' },
        false,
        ['sign']
    );
}

async function deriveSigningKey(
    secretKey: string,
    dateStamp: string,
    region: string,
    service: string
): Promise<CryptoKey> {
    const kSecret = await importHmacKey('AWS4' + secretKey);
    const kDate = await hmacSign(kSecret, dateStamp);
    const kDateKey = await importHmacKey(kDate);
    const kRegion = await hmacSign(kDateKey, region);
    const kRegionKey = await importHmacKey(kRegion);
    const kService = await hmacSign(kRegionKey, service);
    const kServiceKey = await importHmacKey(kService);
    const kSigning = await hmacSign(kServiceKey, 'aws4_request');
    return importHmacKey(kSigning);
}

function encodeRfc3986(str: string): string {
    return encodeURIComponent(str).replace(/[!'()*]/g, c => '%' + c.charCodeAt(0).toString(16).toUpperCase());
}

function buildCanonicalQueryString(params: Record<string, string>): string {
    return Object.entries(params)
        .sort(([a], [b]) => a.localeCompare(b))
        .map(([key, value]) => `${encodeRfc3986(key)}=${encodeRfc3986(value)}`)
        .join('&');
}

interface AwsCredentials {
    accessKeyId: string;
    secretAccessKey: string;
    region: string;
    endpoint: string;
}

async function createPresignedUrl(
    credentials: AwsCredentials,
    params: PresignedUrlParams
): Promise<string> {
    const now = new Date();
    const isoDate = now.toISOString().replace(/[:-]|\.\d{3}/g, '');
    const dateStamp = isoDate.slice(0, 8);

    const endpointUrl = new URL(credentials.endpoint);
    const host = endpointUrl.host;
    const protocol = endpointUrl.protocol;

    const encodedKey = params.key
        .split('/')
        .map(segment => encodeRfc3986(segment))
        .join('/');
    const canonicalUri = `/${params.bucket}/${encodedKey}`;

    const scope = `${dateStamp}/${credentials.region}/${CONFIG.S3_SERVICE}/aws4_request`;

    const signedHeadersList = params.contentType ? ['content-type', 'host'] : ['host'];
    const signedHeaders = signedHeadersList.join(';');

    const queryParams: Record<string, string> = {
        'X-Amz-Algorithm': CONFIG.AWS_SIGNATURE_VERSION,
        'X-Amz-Credential': `${credentials.accessKeyId}/${scope}`,
        'X-Amz-Date': isoDate,
        'X-Amz-Expires': params.expiresInSeconds.toString(),
        'X-Amz-SignedHeaders': signedHeaders,
    };

    const canonicalQueryString = buildCanonicalQueryString(queryParams);

    let canonicalHeaders = '';
    if (params.contentType) {
        canonicalHeaders += `content-type:${params.contentType}\n`;
    }
    canonicalHeaders += `host:${host}\n`;

    const payloadHash = 'UNSIGNED-PAYLOAD';

    const canonicalRequest = [
        params.method,
        canonicalUri,
        canonicalQueryString,
        canonicalHeaders,
        signedHeaders,
        payloadHash,
    ].join('\n');

    const canonicalRequestHash = await sha256Hex(canonicalRequest);

    const stringToSign = [
        CONFIG.AWS_SIGNATURE_VERSION,
        isoDate,
        scope,
        canonicalRequestHash,
    ].join('\n');

    const signingKey = await deriveSigningKey(
        credentials.secretAccessKey,
        dateStamp,
        credentials.region,
        CONFIG.S3_SERVICE
    );

    const signatureBuffer = await hmacSign(signingKey, stringToSign);
    const signature = bufferToHex(signatureBuffer);

    const presignedUrl = `${protocol}//${host}${canonicalUri}?${canonicalQueryString}&X-Amz-Signature=${signature}`;

    return presignedUrl;
}

function handleHealthCheck(ctx: RequestContext, corsHeaders: Record<string, string>): Response {
    return jsonResponse(
        {
            status: 'healthy',
            timestamp: new Date().toISOString(),
            version: '1.0.0',
        },
        200,
        ctx,
        corsHeaders
    );
}

async function handleGenerateUploadUrls(
    request: Request,
    env: Env,
    ctx: RequestContext,
    corsHeaders: Record<string, string>,
    caseId: string
): Promise<Response> {
    if (!validateCaseId(caseId)) {
        throw AppError.badRequest('Invalid case ID format', 'INVALID_CASE_ID');
    }

    const auth = await authenticate(request, env, ctx);

    const hasAccess = await checkCaseAccess(caseId, auth, env, ctx);
    if (!hasAccess) {
        throw AppError.forbidden('You do not have access to this case');
    }

    let rawBody: unknown;
    try {
        rawBody = await request.json();
    } catch {
        throw AppError.badRequest('Invalid JSON body', 'INVALID_JSON');
    }

    const parseResult = UploadRequestSchema.safeParse(rawBody);
    if (!parseResult.success) {
        const firstError = parseResult.error.errors[0];
        const errorMessage = firstError ? `${firstError.path.join('.')} - ${firstError.message}` : 'Unknown validation error';
        throw AppError.badRequest(
            `Validation error: ${errorMessage}`,
            'VALIDATION_ERROR'
        );
    }

    const body = parseResult.data;
    const format = body.format;
    const contentType = ALLOWED_FORMATS[format]!;
    const extension = format === 'jpeg' ? 'jpg' : format;

    const credentials: AwsCredentials = {
        accessKeyId: env.AWS_ACCESS_KEY_ID,
        secretAccessKey: env.AWS_SECRET_ACCESS_KEY,
        region: env.AWS_REGION || 'auto',
        endpoint: env.AWS_ENDPOINT_URL_S3,
    };

    const responseBody: Record<string, UploadUrlResponse> = {};

    for (const kind of body.kinds) {
        const key = `rescues/${caseId}/${kind}.${extension}`;

        const presignedUrl = await createPresignedUrl(credentials, {
            method: 'PUT',
            bucket: env.TIGRIS_BUCKET,
            key,
            expiresInSeconds: CONFIG.UPLOAD_URL_EXPIRY_SECONDS,
            contentType,
        });

        responseBody[kind] = {
            upload_url: presignedUrl,
            object_key: key,
            expires_at: new Date(Date.now() + CONFIG.UPLOAD_URL_EXPIRY_SECONDS * 1000).toISOString(),
            content_type: contentType,
            max_size_bytes: CONFIG.MAX_UPLOAD_SIZE_BYTES,
            required_headers: {
                'Content-Type': contentType,
            },
        };
    }

    log(ctx, 'info', 'upload_urls_generated', {
        user_id: auth.userId,
        case_id: caseId,
        kinds: body.kinds,
        format,
    });

    return jsonResponse(responseBody, 200, ctx, corsHeaders);
}

async function handleGetMediaUrl(
    request: Request,
    env: Env,
    ctx: RequestContext,
    corsHeaders: Record<string, string>,
    caseId: string
): Promise<Response> {
    if (!validateCaseId(caseId)) {
        throw AppError.badRequest('Invalid case ID format', 'INVALID_CASE_ID');
    }

    const auth = await authenticate(request, env, ctx);

    const hasAccess = await checkCaseAccess(caseId, auth, env, ctx);
    if (!hasAccess) {
        throw AppError.forbidden('You do not have access to this case');
    }

    const url = new URL(request.url);
    const kindParam = url.searchParams.get('kind') || 'original';
    const formatParam = url.searchParams.get('format') || 'webp';

    if (!validateKind(kindParam)) {
        throw AppError.badRequest(`Invalid kind: ${sanitizeForLogging(kindParam)}`, 'INVALID_KIND');
    }

    if (!validateFormat(formatParam)) {
        throw AppError.badRequest(`Invalid format: ${sanitizeForLogging(formatParam)}`, 'INVALID_FORMAT');
    }

    const kind = kindParam;
    const extension = formatParam === 'jpeg' ? 'jpg' : formatParam;
    const key = `rescues/${caseId}/${kind}.${extension}`;

    const credentials: AwsCredentials = {
        accessKeyId: env.AWS_ACCESS_KEY_ID,
        secretAccessKey: env.AWS_SECRET_ACCESS_KEY,
        region: env.AWS_REGION || 'auto',
        endpoint: env.AWS_ENDPOINT_URL_S3,
    };

    const presignedUrl = await createPresignedUrl(credentials, {
        method: 'GET',
        bucket: env.TIGRIS_BUCKET,
        key,
        expiresInSeconds: CONFIG.DOWNLOAD_URL_EXPIRY_SECONDS,
    });

    log(ctx, 'info', 'download_url_generated', {
        user_id: auth.userId,
        case_id: caseId,
        kind,
        format: formatParam,
    });

    const response: DownloadUrlResponse = {
        url: presignedUrl,
        expires_at: new Date(Date.now() + CONFIG.DOWNLOAD_URL_EXPIRY_SECONDS * 1000).toISOString(),
        kind,
        format: formatParam,
    };

    return jsonResponse(response, 200, ctx, corsHeaders);
}

interface RouteMatch {
    handler: string;
    params: Record<string, string>;
}

function matchRoute(method: string, pathname: string): RouteMatch | null {
    if (method === 'GET' && pathname === '/health') {
        return { handler: 'health', params: {} };
    }

    if (method === 'GET' && pathname === '/v1/health') {
        return { handler: 'health', params: {} };
    }

    const uploadMatch = pathname.match(/^\/v1\/cases\/([^/]+)\/uploads$/);
    if (method === 'POST' && uploadMatch) {
        return { handler: 'generateUploadUrls', params: { caseId: uploadMatch[1] ?? '' } };
    }

    const mediaMatch = pathname.match(/^\/v1\/cases\/([^/]+)\/media$/);
    if (method === 'GET' && mediaMatch) {
        return { handler: 'getMediaUrl', params: { caseId: mediaMatch[1] ?? '' } };
    }

    return null;
}

export default {
    async fetch(request: Request, env: Env, _ctx: ExecutionContext): Promise<Response> {
        const ctx = createRequestContext(request);
        const origin = request.headers.get('Origin');
        let corsHeaders: Record<string, string> = {};

        try {
            validateEnv(env);
            corsHeaders = getCorsHeaders(env, origin);
        } catch (error) {
            log(ctx, 'error', 'config_error', { error: String(error) });
            return jsonResponse(
                { error: 'Server configuration error', code: 'CONFIG_ERROR', request_id: ctx.requestId },
                500,
                ctx,
                {}
            );
        }

        if (request.method === 'OPTIONS') {
            return handlePreflight(corsHeaders);
        }

        try {
            const rateLimitKey = ctx.ip;
            const rateLimit = await checkRateLimit(env, rateLimitKey, ctx);

            if (!rateLimit.allowed) {
                log(ctx, 'warn', 'rate_limited', { ip: ctx.ip });
                throw AppError.rateLimited(CONFIG.RATE_LIMIT_WINDOW_SECONDS);
            }

            const rateLimitHeaders: Record<string, string> = {
                'X-RateLimit-Remaining': String(rateLimit.remaining),
            };

            const url = new URL(request.url);
            const route = matchRoute(request.method, url.pathname);

            if (!route) {
                throw AppError.notFound('Endpoint not found');
            }

            let response: Response;

            switch (route.handler) {
                case 'health':
                    response = handleHealthCheck(ctx, { ...corsHeaders, ...rateLimitHeaders });
                    break;

                case 'generateUploadUrls':
                    response = await handleGenerateUploadUrls(
                        request,
                        env,
                        ctx,
                        { ...corsHeaders, ...rateLimitHeaders },
                        decodeURIComponent(route.params['caseId'] ?? '')
                    );
                    break;

                case 'getMediaUrl':
                    response = await handleGetMediaUrl(
                        request,
                        env,
                        ctx,
                        { ...corsHeaders, ...rateLimitHeaders },
                        decodeURIComponent(route.params['caseId'] ?? '')
                    );
                    break;

                default:
                    throw AppError.notFound('Endpoint not found');
            }

            log(ctx, 'info', 'request_complete', { status: response.status });
            return response;

        } catch (error) {
            if (error instanceof AppError) {
                log(ctx, error.isOperational ? 'warn' : 'error', 'request_error', {
                    code: error.code,
                    message: error.message,
                    status: error.statusCode,
                });
                return errorResponse(error, ctx, corsHeaders);
            }

            log(ctx, 'error', 'unhandled_error', {
                error: String(error),
                stack: error instanceof Error ? error.stack : undefined,
            });

            return errorResponse(AppError.internal(), ctx, corsHeaders);
        }
    },
};

export const __test__ = {
    validateCaseId,
    validateKind,
    validateFormat,
    buildCanonicalQueryString,
    encodeRfc3986,
    sha256Hex,
    deriveSigningKey,
    createPresignedUrl,
    verifyJWT,
    AppError,
    CONFIG,
    ALLOWED_KINDS,
    ALLOWED_FORMATS,
};