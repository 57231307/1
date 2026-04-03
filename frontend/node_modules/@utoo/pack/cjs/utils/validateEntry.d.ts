import { ConfigComplete } from "../config/types";
/**
 * Validates that all entry.import paths exist in the filesystem.
 * Throws an error if any entry path doesn't exist.
 *
 * @param config - The complete config containing entry points
 * @param projectPath - The project root path to resolve relative entry paths
 * @throws {Error} If any entry.import path doesn't exist
 */
export declare function validateEntryPaths(config: ConfigComplete, projectPath: string): void;
