#!/usr/bin/env npx ts-node
/**
 * Generate TypeScript types from JSON Schema definitions.
 * Run with: npx ts-node scripts/generate-types.ts
 * Or: npm run generate:types
 */

import { compile } from 'json-schema-to-typescript';
import * as fs from 'fs';
import * as path from 'path';

const SCHEMA_DIR = path.resolve(__dirname, '../../shared/schema');
const OUTPUT_DIR = path.resolve(__dirname, '../src/lib/generated');

async function generateFromSchema(schemaPath: string, outputName: string): Promise<void> {
  const schema = JSON.parse(fs.readFileSync(schemaPath, 'utf-8'));

  // Compile schema to TypeScript
  const ts = await compile(schema, schema.title || outputName, {
    bannerComment: `/**
 * Auto-generated from ${path.relative(process.cwd(), schemaPath)}
 * DO NOT EDIT - regenerate with: npm run generate:types
 */`,
    style: {
      singleQuote: true,
      semi: true,
    },
    declareExternallyReferenced: false,
  });

  const outputPath = path.join(OUTPUT_DIR, `${outputName}.d.ts`);
  fs.writeFileSync(outputPath, ts);
  console.log(`Generated: ${outputPath}`);
}

async function main(): Promise<void> {
  // Ensure output directory exists
  if (!fs.existsSync(OUTPUT_DIR)) {
    fs.mkdirSync(OUTPUT_DIR, { recursive: true });
  }

  // Generate types from each schema file
  const schemas = [
    ['types.json', 'types'],
    ['errors.json', 'errors'],
    ['rpc.json', 'rpc'],
    ['methods/core.json', 'methods-core'],
    ['methods/sessions.json', 'methods-sessions'],
    ['methods/panes.json', 'methods-panes'],
    ['methods/events.json', 'methods-events'],
    ['methods/stats.json', 'methods-stats'],
    ['methods/actions.json', 'methods-actions'],
    ['methods/admin.json', 'methods-admin'],
    ['events/notifications.json', 'notifications'],
  ];

  for (const [schemaFile, outputName] of schemas) {
    const schemaPath = path.join(SCHEMA_DIR, schemaFile);
    if (fs.existsSync(schemaPath)) {
      try {
        await generateFromSchema(schemaPath, outputName);
      } catch (err) {
        console.error(`Error generating ${outputName}: ${err}`);
      }
    } else {
      console.warn(`Schema not found: ${schemaPath}`);
    }
  }

  // Generate index file
  const indexContent = schemas
    .filter(([schemaFile]) => fs.existsSync(path.join(SCHEMA_DIR, schemaFile)))
    .map(([_, outputName]) => `export * from './${outputName}';`)
    .join('\n');

  fs.writeFileSync(path.join(OUTPUT_DIR, 'index.d.ts'), indexContent + '\n');
  console.log(`Generated: ${path.join(OUTPUT_DIR, 'index.d.ts')}`);
}

main().catch(console.error);
