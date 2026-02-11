import { S3Client, PutObjectCommand, GetObjectCommand, DeleteObjectCommand } from '@aws-sdk/client-s3';
import { getSignedUrl } from '@aws-sdk/s3-request-presigner';
import { Context } from 'hono';
import { HonoEnv } from '../types';

export const getTigrisClient = (c: Context<HonoEnv>) => {
    return new S3Client({
        region: 'auto',
        endpoint: c.env.TIGRIS_ENDPOINT,
        credentials: {
            accessKeyId: c.env.TIGRIS_ACCESS_KEY_ID,
            secretAccessKey: c.env.TIGRIS_SECRET_ACCESS_KEY,
        },
    });
};

/**
 * Generate a signed PUT URL for uploading media
 * @param client S3 client instance
 * @param bucket Tigris bucket name
 * @param objectKey Object key (e.g., "rescues/{case_id}/original.webp")
 * @param ttl Time-to-live in seconds (default: 900s / 15min)
 */
export async function generateSignedPutUrl(
    client: S3Client,
    bucket: string,
    objectKey: string,
    ttl: number = 900
): Promise<string> {
    const command = new PutObjectCommand({
        Bucket: bucket,
        Key: objectKey,
        ContentType: 'image/webp', // WarmStreet standard
    });

    return await getSignedUrl(client, command, { expiresIn: ttl });
}

/**
 * Generate a signed GET URL for downloading/viewing media
 * @param client S3 client instance
 * @param bucket Tigris bucket name
 * @param objectKey Object key
 * @param ttl Time-to-live in seconds (default: 300s / 5min)
 */
export async function generateSignedGetUrl(
    client: S3Client,
    bucket: string,
    objectKey: string,
    ttl: number = 300
): Promise<string> {
    const command = new GetObjectCommand({
        Bucket: bucket,
        Key: objectKey,
    });

    return await getSignedUrl(client, command, { expiresIn: ttl });
}

/**
 * Delete an object from Tigris (cleanup)
 * @param client S3 client instance
 * @param bucket Tigris bucket name
 * @param objectKey Object key to delete
 */
export async function deleteObject(
    client: S3Client,
    bucket: string,
    objectKey: string
): Promise<void> {
    const command = new DeleteObjectCommand({
        Bucket: bucket,
        Key: objectKey,
    });

    await client.send(command);
}
