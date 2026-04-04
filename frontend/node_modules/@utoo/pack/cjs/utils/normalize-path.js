"use strict";
var __importDefault = (this && this.__importDefault) || function (mod) {
    return (mod && mod.__esModule) ? mod : { "default": mod };
};
Object.defineProperty(exports, "__esModule", { value: true });
exports.normalizePath = normalizePath;
const path_1 = __importDefault(require("path"));
function normalizePath(file) {
    return path_1.default.sep === "\\" ? file.replace(/\\/g, "/") : file;
}
