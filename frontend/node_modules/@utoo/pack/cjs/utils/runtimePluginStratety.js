"use strict";
var __importDefault = (this && this.__importDefault) || function (mod) {
    return (mod && mod.__esModule) ? mod : { "default": mod };
};
Object.defineProperty(exports, "__esModule", { value: true });
exports.useWorkerThreads = useWorkerThreads;
const semver_1 = __importDefault(require("semver"));
function useWorkerThreads() {
    const WORKER_THREADS_SAFE_RANGE = ">=24.13.1 <25.0.0 || >=25.4.0";
    return semver_1.default.satisfies(process.version, WORKER_THREADS_SAFE_RANGE, {
        includePrerelease: true,
    });
}
