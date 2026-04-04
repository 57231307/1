import { BundleOptions } from "../config/types";
import { WebpackConfig } from "../config/webpackCompat";
export declare function build(options: BundleOptions | WebpackConfig, projectPath?: string, rootPath?: string): Promise<void>;
