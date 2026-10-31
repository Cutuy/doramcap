import { Root, Type, util, type Field } from "protobufjs/light";
import schema from "./generated/schema.json";

export const root = Root.fromJSON(schema).resolveAll();
const own = (o: object, k: string) => Object.hasOwn(o, k);
export type Document = Record<string, any>;
export function object(value: unknown, context: string): Document {
  if (
    !value ||
    typeof value !== "object" ||
    Array.isArray(value) ||
    JSON.isRawJSON(value)
  )
    throw new Error(`invalid ${context}`);
  return value as Document;
}
export function parse(bytes: Uint8Array): Document {
  return object(
    JSON.parse(util.utf8.read(bytes, 0, bytes.length), (_k, v, context) =>
      typeof v === "number" ? JSON.rawJSON(context.source) : v,
    ),
    "Compose document",
  );
}
export const numeric = (v: any) => (JSON.isRawJSON(v) ? v.rawJSON : String(v));
export const integer = (v: any) => JSON.rawJSON(numeric(v));
export function textBytes(value: string): Uint8Array {
  const bytes = new Uint8Array(util.utf8.length(value));
  util.utf8.write(value, bytes, 0);
  return bytes;
}
export function encode(name: string, value: object): Uint8Array {
  const type = root.lookupType(name);
  const message = type.fromObject(value);
  const error = type.verify(message);
  if (error) throw new Error(`${name}: ${error}`);
  return type.encode(message).finish();
}

// Only foreign fields use this recursive representation; numbers retain their exact spelling.
function native(value: any): Document {
  if (value === null) return { null: 0 };
  if (JSON.isRawJSON(value) || typeof value === "number")
    return { number: numeric(value) };
  if (Array.isArray(value)) return { list: { values: value.map(native) } };
  if (typeof value === "object")
    return {
      object: {
        fields: Object.fromEntries(
          Object.entries(value).map(([k, v]) => [k, native(v)]),
        ),
      },
    };
  return typeof value === "boolean" ? { boolean: value } : { text: value };
}
function restore(value: Document): any {
  if (own(value, "null")) return null;
  if (own(value, "number")) return JSON.rawJSON(value.number);
  if (own(value, "boolean")) return value.boolean;
  if (own(value, "text")) return value.text;
  if (value.list) return (value.list.values ?? []).map(restore);
  if (value.object)
    return Object.fromEntries(
      Object.entries(value.object.fields ?? {}).map(([k, v]) => [
        k,
        restore(v as Document),
      ]),
    );
  throw new Error("foreign value has no selected variant");
}
function integerText(value: any, bits: number, signed: boolean): string {
  const text = numeric(value);
  if (!/^-?\d+$/.test(text)) throw new Error("invalid integer value");
  const number = BigInt(text);
  const maximum = (1n << BigInt(bits - (signed ? 1 : 0))) - 1n;
  const minimum = signed ? -(maximum + 1n) : 0n;
  if (number < minimum || number > maximum)
    throw new Error("integer exceeds protobuf field range");
  return text;
}
function scalar(field: Field, value: any): any {
  if (field.resolvedType instanceof Type)
    return toProto(field.resolvedType, value);
  if (["uint64", "int64"].includes(field.type))
    return integerText(value, 64, field.type === "int64");
  if (["uint32", "int32"].includes(field.type))
    return Number(integerText(value, 32, field.type === "int32"));
  return value;
}
export function toProto(type: Type, value: any): Document {
  switch (type.name) {
    case "ComposeNetworkAttachment":
      if (value === null) return { implicit: 0 };
      break;
    case "ComposeCommand":
      return value === null
        ? { inherit: 0 }
        : typeof value === "string"
          ? { text: value }
          : { argv: { values: value } };
    case "EnvironmentValue":
      return value === null ? { unset: 0 } : { text: value };
    case "ComposeUlimit":
      return JSON.isRawJSON(value) || typeof value === "number"
        ? { limit: integerText(value, 64, true) }
        : {
            range: {
              soft: integerText(value.soft, 64, true),
              hard: integerText(value.hard, 64, true),
            },
          };
  }
  const source = object(value, type.name);
  const result: Document = Object.create(null);
  for (const [key, item] of Object.entries(source)) {
    const field = own(type.fields, key) && type.fields[key];
    if (!field || key === "native_fields") {
      result.native_fields ??= Object.create(null);
      result.native_fields[key] = native(item);
    } else if (field.map) {
      result[key] = Object.fromEntries(
        Object.entries(object(item, key)).map(([k, v]) => [
          k,
          scalar(field, v),
        ]),
      );
    } else if (field.repeated) {
      if (!Array.isArray(item)) throw new Error(`invalid ${key}`);
      result[key] = item.map((v) => scalar(field, v));
    } else result[key] = scalar(field, item);
  }
  return result;
}
function output(field: Field, value: any): any {
  if (field.resolvedType instanceof Type)
    return fromProto(field.resolvedType, value);
  if (["uint64", "int64"].includes(field.type)) return integer(value);
  return value;
}
export function fromProto(type: Type, value: Document): any {
  switch (type.name) {
    case "ComposeNetworkAttachment":
      if (own(value, "implicit")) return null;
      break;
    case "ComposeCommand":
      return own(value, "inherit")
        ? null
        : own(value, "text")
          ? value.text
          : (value.argv?.values ?? []);
    case "EnvironmentValue":
      return own(value, "unset") ? null : value.text;
    case "ComposeUlimit":
      return own(value, "limit")
        ? integer(value.limit)
        : { soft: integer(value.range.soft), hard: integer(value.range.hard) };
  }
  const result: Document = Object.fromEntries(
    Object.entries(value.native_fields ?? {}).map(([k, v]) => {
      if (own(type.fields, k) && k !== "native_fields")
        throw new Error(`foreign field shadows ${type.name}.${k}`);
      return [k, restore(v as Document)];
    }),
  );
  for (const field of type.fieldsArray) {
    if (field.name === "native_fields" || !own(value, field.name)) continue;
    const item = value[field.name];
    if (field.map) {
      if (Object.keys(item).length)
        result[field.name] = Object.fromEntries(
          Object.entries(item).map(([k, v]) => [k, output(field, v)]),
        );
    } else if (field.repeated) {
      if (item.length)
        result[field.name] = item.map((v: any) => output(field, v));
    } else result[field.name] = output(field, item);
  }
  return result;
}
