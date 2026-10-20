import * as $protobuf from "protobufjs";
import Long = require("long");

/** Namespace doramcap. */
export namespace doramcap {

    /** Namespace activity. */
    namespace activity {

        /** Namespace v1. */
        namespace v1 {

            /**
             * Properties of a Batch.
             * @deprecated Use doramcap.activity.v1.Batch.$Properties instead.
             */
            interface IBatch extends doramcap.activity.v1.Batch.$Properties {
            }

            /** Represents a Batch. */
            class Batch {

                /**
                 * Constructs a new Batch.
                 * @param [properties] Properties to set
                 */
                constructor(properties?: doramcap.activity.v1.Batch.$Properties);

                /** Unknown fields preserved while decoding when enabled */
                $unknowns?: Uint8Array[];

                /** Batch version. */
                version: number;

                /** Batch session. */
                session: (number|Long);

                /** Batch sequence. */
                sequence: (number|Long);

                /** Batch unixOriginNs. */
                unixOriginNs: (number|Long);

                /** Batch droppedRecords. */
                droppedRecords: (number|Long);

                /** Batch pid. */
                pid: number;

                /** Batch dictionary. */
                dictionary: string[];

                /** Batch strings. */
                strings: Uint8Array;

                /** Batch records. */
                records: doramcap.activity.v1.Record.$Properties[];

                /** Batch droppedFields. */
                droppedFields: (number|Long);

                /** Batch rejectedSpans. */
                rejectedSpans: (number|Long);

                /** Batch unregisteredRecords. */
                unregisteredRecords: (number|Long);

                /**
                 * Encodes the specified Batch message. Does not implicitly {@link doramcap.activity.v1.Batch.verify|verify} messages.
                 * @param message Batch message or plain object to encode
                 * @param [writer] Writer to encode to
                 * @returns Writer
                 */
                static encode(message: doramcap.activity.v1.Batch.$Properties, writer?: $protobuf.Writer): $protobuf.Writer;

                /**
                 * Decodes a Batch message from the specified reader or buffer.
                 * @param reader Reader or buffer to decode from
                 * @param [length] Message length if known beforehand
                 * @returns {doramcap.activity.v1.Batch & doramcap.activity.v1.Batch.$Shape} Batch
                 * @throws {Error} If the payload is not a reader or valid buffer
                 * @throws {$protobuf.util.ProtocolError} If required fields are missing
                 */
                static decode(reader: ($protobuf.Reader|Uint8Array), length?: number): doramcap.activity.v1.Batch & doramcap.activity.v1.Batch.$Shape;

                /**
                 * Creates a Batch message from a plain object. Also converts values to their respective internal types.
                 * @param object Plain object
                 * @returns Batch
                 */
                static fromObject(object: { [k: string]: any }): doramcap.activity.v1.Batch;

                /**
                 * Creates a plain object from a Batch message. Also converts values to other types if specified.
                 * @param message Batch
                 * @param [options] Conversion options
                 * @returns Plain object
                 */
                static toObject(message: doramcap.activity.v1.Batch, options?: $protobuf.IConversionOptions): { [k: string]: any };

                /**
                 * Converts this Batch to JSON.
                 * @returns JSON object
                 */
                toJSON(): { [k: string]: any };

                /**
                 * Gets the type url for Batch
                 * @param [prefix] Custom type url prefix, defaults to `"type.googleapis.com"`
                 * @returns The type url
                 */
                static getTypeUrl(prefix?: string): string;
            }

            namespace Batch {

                /** Properties of a Batch. */
                interface $Properties {

                    /** Batch version */
                    version?: (number|null);

                    /** Batch session */
                    session?: (number|Long|null);

                    /** Batch sequence */
                    sequence?: (number|Long|null);

                    /** Batch unixOriginNs */
                    unixOriginNs?: (number|Long|null);

                    /** Batch droppedRecords */
                    droppedRecords?: (number|Long|null);

                    /** Batch pid */
                    pid?: (number|null);

                    /** Batch dictionary */
                    dictionary?: (string[]|null);

                    /** Batch strings */
                    strings?: (Uint8Array|null);

                    /** Batch records */
                    records?: (doramcap.activity.v1.Record.$Properties[]|null);

                    /** Batch droppedFields */
                    droppedFields?: (number|Long|null);

                    /** Batch rejectedSpans */
                    rejectedSpans?: (number|Long|null);

                    /** Batch unregisteredRecords */
                    unregisteredRecords?: (number|Long|null);

                    /** Unknown fields preserved while decoding when enabled */
                    $unknowns?: Uint8Array[];
                }

                /** Shape of a Batch. */
                type $Shape = doramcap.activity.v1.Batch.$Properties;
            }

            /**
             * Properties of a Record.
             * @deprecated Use doramcap.activity.v1.Record.$Properties instead.
             */
            interface IRecord extends doramcap.activity.v1.Record.$Properties {
            }

            /** Represents a Record. */
            class Record {

                /**
                 * Constructs a new Record.
                 * @param [properties] Properties to set
                 */
                constructor(properties?: doramcap.activity.v1.Record.$Properties);

                /** Unknown fields preserved while decoding when enabled */
                $unknowns?: Uint8Array[];

                /** Record kind. */
                kind: doramcap.activity.v1.Record.Kind;

                /** Record monotonicNs. */
                monotonicNs: (number|Long);

                /** Record spanId. */
                spanId: (number|Long);

                /** Record traceRoot. */
                traceRoot: (number|Long);

                /** Record parentId. */
                parentId: (number|Long);

                /** Record linkedId. */
                linkedId: (number|Long);

                /** Record linkedRoot. */
                linkedRoot: (number|Long);

                /** Record tid. */
                tid: number;

                /** Record threadName. */
                threadName: number;

                /** Record name. */
                name: number;

                /** Record fields. */
                fields: doramcap.activity.v1.Field.$Properties[];

                /**
                 * Encodes the specified Record message. Does not implicitly {@link doramcap.activity.v1.Record.verify|verify} messages.
                 * @param message Record message or plain object to encode
                 * @param [writer] Writer to encode to
                 * @returns Writer
                 */
                static encode(message: doramcap.activity.v1.Record.$Properties, writer?: $protobuf.Writer): $protobuf.Writer;

                /**
                 * Decodes a Record message from the specified reader or buffer.
                 * @param reader Reader or buffer to decode from
                 * @param [length] Message length if known beforehand
                 * @returns {doramcap.activity.v1.Record & doramcap.activity.v1.Record.$Shape} Record
                 * @throws {Error} If the payload is not a reader or valid buffer
                 * @throws {$protobuf.util.ProtocolError} If required fields are missing
                 */
                static decode(reader: ($protobuf.Reader|Uint8Array), length?: number): doramcap.activity.v1.Record & doramcap.activity.v1.Record.$Shape;

                /**
                 * Creates a Record message from a plain object. Also converts values to their respective internal types.
                 * @param object Plain object
                 * @returns Record
                 */
                static fromObject(object: { [k: string]: any }): doramcap.activity.v1.Record;

                /**
                 * Creates a plain object from a Record message. Also converts values to other types if specified.
                 * @param message Record
                 * @param [options] Conversion options
                 * @returns Plain object
                 */
                static toObject(message: doramcap.activity.v1.Record, options?: $protobuf.IConversionOptions): { [k: string]: any };

                /**
                 * Converts this Record to JSON.
                 * @returns JSON object
                 */
                toJSON(): { [k: string]: any };

                /**
                 * Gets the type url for Record
                 * @param [prefix] Custom type url prefix, defaults to `"type.googleapis.com"`
                 * @returns The type url
                 */
                static getTypeUrl(prefix?: string): string;
            }

            namespace Record {

                /** Properties of a Record. */
                interface $Properties {

                    /** Record kind */
                    kind?: (doramcap.activity.v1.Record.Kind|null);

                    /** Record monotonicNs */
                    monotonicNs?: (number|Long|null);

                    /** Record spanId */
                    spanId?: (number|Long|null);

                    /** Record traceRoot */
                    traceRoot?: (number|Long|null);

                    /** Record parentId */
                    parentId?: (number|Long|null);

                    /** Record linkedId */
                    linkedId?: (number|Long|null);

                    /** Record linkedRoot */
                    linkedRoot?: (number|Long|null);

                    /** Record tid */
                    tid?: (number|null);

                    /** Record threadName */
                    threadName?: (number|null);

                    /** Record name */
                    name?: (number|null);

                    /** Record fields */
                    fields?: (doramcap.activity.v1.Field.$Properties[]|null);

                    /** Unknown fields preserved while decoding when enabled */
                    $unknowns?: Uint8Array[];
                }

                /** Shape of a Record. */
                type $Shape = doramcap.activity.v1.Record.$Properties;

                /** Kind enum. */
                enum Kind {

                    /** BEGIN value */
                    BEGIN = 0,

                    /** EVENT value */
                    EVENT = 1,

                    /** UPDATE value */
                    UPDATE = 2,

                    /** LINK value */
                    LINK = 3,

                    /** END value */
                    END = 4
                }
            }

            /**
             * Properties of a Field.
             * @deprecated Use doramcap.activity.v1.Field.$Properties instead.
             */
            interface IField extends doramcap.activity.v1.Field.$Properties {
            }

            /** Represents a Field. */
            class Field {

                /**
                 * Constructs a new Field.
                 * @param [properties] Properties to set
                 */
                constructor(properties?: doramcap.activity.v1.Field.$Properties);

                /** Unknown fields preserved while decoding when enabled */
                $unknowns?: Uint8Array[];

                /** Field name. */
                name: number;

                /** Field kind. */
                kind: doramcap.activity.v1.Field.Kind;

                /** Field bits. */
                bits: (number|Long);

                /** Field offset. */
                offset: number;

                /** Field length. */
                length: number;

                /**
                 * Encodes the specified Field message. Does not implicitly {@link doramcap.activity.v1.Field.verify|verify} messages.
                 * @param message Field message or plain object to encode
                 * @param [writer] Writer to encode to
                 * @returns Writer
                 */
                static encode(message: doramcap.activity.v1.Field.$Properties, writer?: $protobuf.Writer): $protobuf.Writer;

                /**
                 * Decodes a Field message from the specified reader or buffer.
                 * @param reader Reader or buffer to decode from
                 * @param [length] Message length if known beforehand
                 * @returns {doramcap.activity.v1.Field & doramcap.activity.v1.Field.$Shape} Field
                 * @throws {Error} If the payload is not a reader or valid buffer
                 * @throws {$protobuf.util.ProtocolError} If required fields are missing
                 */
                static decode(reader: ($protobuf.Reader|Uint8Array), length?: number): doramcap.activity.v1.Field & doramcap.activity.v1.Field.$Shape;

                /**
                 * Creates a Field message from a plain object. Also converts values to their respective internal types.
                 * @param object Plain object
                 * @returns Field
                 */
                static fromObject(object: { [k: string]: any }): doramcap.activity.v1.Field;

                /**
                 * Creates a plain object from a Field message. Also converts values to other types if specified.
                 * @param message Field
                 * @param [options] Conversion options
                 * @returns Plain object
                 */
                static toObject(message: doramcap.activity.v1.Field, options?: $protobuf.IConversionOptions): { [k: string]: any };

                /**
                 * Converts this Field to JSON.
                 * @returns JSON object
                 */
                toJSON(): { [k: string]: any };

                /**
                 * Gets the type url for Field
                 * @param [prefix] Custom type url prefix, defaults to `"type.googleapis.com"`
                 * @returns The type url
                 */
                static getTypeUrl(prefix?: string): string;
            }

            namespace Field {

                /** Properties of a Field. */
                interface $Properties {

                    /** Field name */
                    name?: (number|null);

                    /** Field kind */
                    kind?: (doramcap.activity.v1.Field.Kind|null);

                    /** Field bits */
                    bits?: (number|Long|null);

                    /** Field offset */
                    offset?: (number|null);

                    /** Field length */
                    length?: (number|null);

                    /** Unknown fields preserved while decoding when enabled */
                    $unknowns?: Uint8Array[];
                }

                /** Shape of a Field. */
                type $Shape = doramcap.activity.v1.Field.$Properties;

                /** Kind enum. */
                enum Kind {

                    /** UNSIGNED value */
                    UNSIGNED = 0,

                    /** SIGNED value */
                    SIGNED = 1,

                    /** FLOAT value */
                    FLOAT = 2,

                    /** BOOL value */
                    BOOL = 3,

                    /** STRING value */
                    STRING = 4
                }
            }
        }
    }
}
