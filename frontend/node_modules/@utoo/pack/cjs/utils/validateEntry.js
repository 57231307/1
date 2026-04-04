"use strict";
var __importDefault = (this && this.__importDefault) || function (mod) {
    return (mod && mod.__esModule) ? mod : { "default": mod };
};
Object.defineProperty(exports, "__esModule", { value: true });
exports.validateEntryPaths = validateEntryPaths;
const fs_1 = __importDefault(require("fs"));
const path_1 = __importDefault(require("path"));
/**
 * Validates that all entry.import paths exist in the filesystem.
 * Throws an error if any entry path doesn't exist.
 *
 * @param config - The complete config containing entry points
 * @param projectPath - The project root path to resolve relative entry paths
 * @throws {Error} If any entry.import path doesn't exist
 */
function validateEntryPaths(config, projectPath) {
    if (!config.entry || config.entry.length === 0) {
        return;
    }
    const errors = [];
    for (const entry of config.entry) {
        if (!entry.import) {
            errors.push(`Entry is missing 'import' property`);
            continue;
        }
        // Resolve the entry path relative to projectPath
        const entryPath = path_1.default.isAbsolute(entry.import)
            ? entry.import
            : path_1.default.resolve(projectPath, entry.import);
        // Check if the file exists
        if (!fs_1.default.existsSync(entryPath)) {
            const entryName = entry.name ? ` (name: "${entry.name}")` : "";
            errors.push(`Entry import path does not exist: "${entry.import}"${entryName}\n` +
                `  Resolved path: ${entryPath}`);
        }
    }
    if (errors.length > 0) {
        throw new Error(`Invalid entry configuration:\n${errors.map((e) => `  - ${e}`).join("\n")}`);
    }
}
