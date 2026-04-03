"use strict";
var __importDefault = (this && this.__importDefault) || function (mod) {
    return (mod && mod.__esModule) ? mod : { "default": mod };
};
Object.defineProperty(exports, "__esModule", { value: true });
exports.renderStyledStringToErrorAnsi = renderStyledStringToErrorAnsi;
const picocolors_1 = __importDefault(require("picocolors"));
const magicIdentifier_1 = require("./magicIdentifier");
// picocolors is a CommonJS module, so we need to import it as a default import
const { bold, green, magenta, red } = picocolors_1.default;
function renderStyledStringToErrorAnsi(string) {
    function decodeMagicIdentifiers(str) {
        return str.replaceAll(magicIdentifier_1.MAGIC_IDENTIFIER_REGEX, (ident) => {
            try {
                return magenta(`{${(0, magicIdentifier_1.decodeMagicIdentifier)(ident)}}`);
            }
            catch (e) {
                return magenta(`{${ident} (decoding failed: ${e})}`);
            }
        });
    }
    switch (string.type) {
        case "text":
            return decodeMagicIdentifiers(string.value);
        case "strong":
            return bold(red(decodeMagicIdentifiers(string.value)));
        case "code":
            return green(decodeMagicIdentifiers(string.value));
        case "line":
            return string.value.map(renderStyledStringToErrorAnsi).join("");
        case "stack":
            return string.value.map(renderStyledStringToErrorAnsi).join("\n");
        default:
            throw new Error("Unknown StyledString type", string);
    }
}
