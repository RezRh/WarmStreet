import { GoogleGenerativeAI } from '@google/generative-ai';

export interface GeminiDiagnosis {
    species: string;
    wound_description: string;
    severity_1_10: number;
    urgency: 'low' | 'medium' | 'high' | 'critical';
    care_instructions: string;
}

export class GeminiClient {
    private genAI: GoogleGenerativeAI;

    constructor(apiKey: string) {
        this.genAI = new GoogleGenerativeAI(apiKey);
    }

    /**
     * Analyze an image of an animal wound using Gemini 1.5 Pro/Flash
     * @param imageBuffer WebP image data
     * @param userDescription Optional user-provided context
     */
    async analyzeWound(
        imageBuffer: Buffer | Uint8Array,
        userDescription?: string
    ): Promise<GeminiDiagnosis> {
        const model = this.genAI.getGenerativeModel({ model: 'gemini-2.0-flash' });

        const prompt = `
            Analyze this image of an animal that appears to be injured or in distress.
            
            User Description: ${userDescription || 'No description provided.'}
            
            Provide a structured JSON response with the following fields:
            1. species: The likely species of the animal.
            2. wound_description: A concise medical-style description of the observed injuries.
            3. severity_1_10: An integer from 1 to 10.
            4. urgency: One of 'low', 'medium', 'high', 'critical'.
            5. care_instructions: Brief, safe first-aid instructions for a non-expert bystander.
            
            Return ONLY the JSON object.
        `;

        const result = await model.generateContent([
            prompt,
            {
                inlineData: {
                    data: Buffer.from(imageBuffer).toString('base64'),
                    mimeType: 'image/webp'
                }
            }
        ]);

        const response = await result.response;
        const text = response.text();

        // Extract JSON from response (handling potential markdown blocks)
        try {
            const jsonStr = text.match(/\{[\s\S]*\}/)?.[0] || text;
            return JSON.parse(jsonStr) as GeminiDiagnosis;
        } catch (e) {
            console.error('Failed to parse Gemini response:', text);
            throw new Error('Invalid AI response format');
        }
    }
}
