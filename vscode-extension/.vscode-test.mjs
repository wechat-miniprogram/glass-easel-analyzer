import path from 'path'
import { fileURLToPath } from 'url'
import { defineConfig } from '@vscode/test-cli'

const __filename = fileURLToPath(import.meta.url)
const __dirname = path.dirname(__filename)

export default defineConfig({
	files: 'out/test/**/*.test.js',
	extensionDevelopmentPath: __dirname,
	workspaceFolder: `${__dirname}/test-fixture`,
	mocha: {
		timeout: 20000,
	},
	env: {
		GLASS_EASEL_ANALYZER_SERVER: `${__dirname}/../target/debug/glass-easel-analyzer`,
	},
})
