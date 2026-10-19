import * as $protobuf from "protobufjs";
import Long = require("long");

/** Namespace doramcap. */
export namespace doramcap {

    /** Namespace telemetry. */
    namespace telemetry {

        /** Namespace v1. */
        namespace v1 {

            /**
             * Properties of a Packet.
             * @deprecated Use doramcap.telemetry.v1.Packet.$Properties instead.
             */
            interface IPacket extends doramcap.telemetry.v1.Packet.$Properties {
            }

            /** Represents a Packet. */
            class Packet {

                /**
                 * Constructs a new Packet.
                 * @param [properties] Properties to set
                 */
                constructor(properties?: doramcap.telemetry.v1.Packet.$Properties);

                /** Unknown fields preserved while decoding when enabled */
                $unknowns?: Uint8Array[];

                /** Packet snapshot. */
                snapshot?: (doramcap.telemetry.v1.SnapshotFrame.$Properties|null);

                /** Packet dictionary. */
                dictionary?: (doramcap.telemetry.v1.DictionaryFrame.$Properties|null);

                /** Packet body. */
                body?: ("snapshot"|"dictionary");

                /**
                 * Encodes the specified Packet message. Does not implicitly {@link doramcap.telemetry.v1.Packet.verify|verify} messages.
                 * @param message Packet message or plain object to encode
                 * @param [writer] Writer to encode to
                 * @returns Writer
                 */
                static encode(message: doramcap.telemetry.v1.Packet.$Properties, writer?: $protobuf.Writer): $protobuf.Writer;

                /**
                 * Decodes a Packet message from the specified reader or buffer.
                 * @param reader Reader or buffer to decode from
                 * @param [length] Message length if known beforehand
                 * @returns {doramcap.telemetry.v1.Packet & doramcap.telemetry.v1.Packet.$Shape} Packet
                 * @throws {Error} If the payload is not a reader or valid buffer
                 * @throws {$protobuf.util.ProtocolError} If required fields are missing
                 */
                static decode(reader: ($protobuf.Reader|Uint8Array), length?: number): doramcap.telemetry.v1.Packet & doramcap.telemetry.v1.Packet.$Shape;

                /**
                 * Creates a Packet message from a plain object. Also converts values to their respective internal types.
                 * @param object Plain object
                 * @returns Packet
                 */
                static fromObject(object: { [k: string]: any }): doramcap.telemetry.v1.Packet;

                /**
                 * Creates a plain object from a Packet message. Also converts values to other types if specified.
                 * @param message Packet
                 * @param [options] Conversion options
                 * @returns Plain object
                 */
                static toObject(message: doramcap.telemetry.v1.Packet, options?: $protobuf.IConversionOptions): { [k: string]: any };

                /**
                 * Converts this Packet to JSON.
                 * @returns JSON object
                 */
                toJSON(): { [k: string]: any };

                /**
                 * Gets the type url for Packet
                 * @param [prefix] Custom type url prefix, defaults to `"type.googleapis.com"`
                 * @returns The type url
                 */
                static getTypeUrl(prefix?: string): string;
            }

            namespace Packet {

                /** Properties of a Packet. */
                interface $Properties {

                    /** Packet snapshot */
                    snapshot?: (doramcap.telemetry.v1.SnapshotFrame.$Properties|null);

                    /** Packet dictionary */
                    dictionary?: (doramcap.telemetry.v1.DictionaryFrame.$Properties|null);

                    /** Packet body */
                    body?: ("snapshot"|"dictionary");

                    /** Unknown fields preserved while decoding when enabled */
                    $unknowns?: Uint8Array[];
                }

                /** Narrowed shape of a Packet. */
                type $Shape = {
                  snapshot?: doramcap.telemetry.v1.SnapshotFrame.$Shape|null;
                  dictionary?: doramcap.telemetry.v1.DictionaryFrame.$Shape|null;
                  $unknowns?: Uint8Array[];
                } & (
                  ({ body?: undefined; snapshot?: null; dictionary?: null }|{ body?: "snapshot"; snapshot: doramcap.telemetry.v1.SnapshotFrame.$Shape; dictionary?: null }|{ body?: "dictionary"; snapshot?: null; dictionary: doramcap.telemetry.v1.DictionaryFrame.$Shape })
                );
            }

            /**
             * Properties of a CpuRecord.
             * @deprecated Use doramcap.telemetry.v1.CpuRecord.$Properties instead.
             */
            interface ICpuRecord extends doramcap.telemetry.v1.CpuRecord.$Properties {
            }

            /** Represents a CpuRecord. */
            class CpuRecord {

                /**
                 * Constructs a new CpuRecord.
                 * @param [properties] Properties to set
                 */
                constructor(properties?: doramcap.telemetry.v1.CpuRecord.$Properties);

                /** Unknown fields preserved while decoding when enabled */
                $unknowns?: Uint8Array[];

                /** CpuRecord cgroupId. */
                cgroupId: (number|Long);

                /** CpuRecord parentId. */
                parentId: (number|Long);

                /** CpuRecord cpu. */
                cpu: number;

                /** CpuRecord flags. */
                flags: number;

                /** CpuRecord runtimeNs. */
                runtimeNs: (number|Long);

                /** CpuRecord userNs. */
                userNs: (number|Long);

                /** CpuRecord systemNs. */
                systemNs: (number|Long);

                /**
                 * Encodes the specified CpuRecord message. Does not implicitly {@link doramcap.telemetry.v1.CpuRecord.verify|verify} messages.
                 * @param message CpuRecord message or plain object to encode
                 * @param [writer] Writer to encode to
                 * @returns Writer
                 */
                static encode(message: doramcap.telemetry.v1.CpuRecord.$Properties, writer?: $protobuf.Writer): $protobuf.Writer;

                /**
                 * Decodes a CpuRecord message from the specified reader or buffer.
                 * @param reader Reader or buffer to decode from
                 * @param [length] Message length if known beforehand
                 * @returns {doramcap.telemetry.v1.CpuRecord & doramcap.telemetry.v1.CpuRecord.$Shape} CpuRecord
                 * @throws {Error} If the payload is not a reader or valid buffer
                 * @throws {$protobuf.util.ProtocolError} If required fields are missing
                 */
                static decode(reader: ($protobuf.Reader|Uint8Array), length?: number): doramcap.telemetry.v1.CpuRecord & doramcap.telemetry.v1.CpuRecord.$Shape;

                /**
                 * Creates a CpuRecord message from a plain object. Also converts values to their respective internal types.
                 * @param object Plain object
                 * @returns CpuRecord
                 */
                static fromObject(object: { [k: string]: any }): doramcap.telemetry.v1.CpuRecord;

                /**
                 * Creates a plain object from a CpuRecord message. Also converts values to other types if specified.
                 * @param message CpuRecord
                 * @param [options] Conversion options
                 * @returns Plain object
                 */
                static toObject(message: doramcap.telemetry.v1.CpuRecord, options?: $protobuf.IConversionOptions): { [k: string]: any };

                /**
                 * Converts this CpuRecord to JSON.
                 * @returns JSON object
                 */
                toJSON(): { [k: string]: any };

                /**
                 * Gets the type url for CpuRecord
                 * @param [prefix] Custom type url prefix, defaults to `"type.googleapis.com"`
                 * @returns The type url
                 */
                static getTypeUrl(prefix?: string): string;
            }

            namespace CpuRecord {

                /** Properties of a CpuRecord. */
                interface $Properties {

                    /** CpuRecord cgroupId */
                    cgroupId?: (number|Long|null);

                    /** CpuRecord parentId */
                    parentId?: (number|Long|null);

                    /** CpuRecord cpu */
                    cpu?: (number|null);

                    /** CpuRecord flags */
                    flags?: (number|null);

                    /** CpuRecord runtimeNs */
                    runtimeNs?: (number|Long|null);

                    /** CpuRecord userNs */
                    userNs?: (number|Long|null);

                    /** CpuRecord systemNs */
                    systemNs?: (number|Long|null);

                    /** Unknown fields preserved while decoding when enabled */
                    $unknowns?: Uint8Array[];
                }

                /** Shape of a CpuRecord. */
                type $Shape = doramcap.telemetry.v1.CpuRecord.$Properties;
            }

            /**
             * Properties of a TaskIdentity.
             * @deprecated Use doramcap.telemetry.v1.TaskIdentity.$Properties instead.
             */
            interface ITaskIdentity extends doramcap.telemetry.v1.TaskIdentity.$Properties {
            }

            /** Represents a TaskIdentity. */
            class TaskIdentity {

                /**
                 * Constructs a new TaskIdentity.
                 * @param [properties] Properties to set
                 */
                constructor(properties?: doramcap.telemetry.v1.TaskIdentity.$Properties);

                /** Unknown fields preserved while decoding when enabled */
                $unknowns?: Uint8Array[];

                /** TaskIdentity startBoottimeNs. */
                startBoottimeNs: (number|Long);

                /** TaskIdentity cgroupId. */
                cgroupId: (number|Long);

                /** TaskIdentity tid. */
                tid: number;

                /** TaskIdentity tgid. */
                tgid: number;

                /** TaskIdentity cpu. */
                cpu: number;

                /** TaskIdentity flags. */
                flags: number;

                /**
                 * Encodes the specified TaskIdentity message. Does not implicitly {@link doramcap.telemetry.v1.TaskIdentity.verify|verify} messages.
                 * @param message TaskIdentity message or plain object to encode
                 * @param [writer] Writer to encode to
                 * @returns Writer
                 */
                static encode(message: doramcap.telemetry.v1.TaskIdentity.$Properties, writer?: $protobuf.Writer): $protobuf.Writer;

                /**
                 * Decodes a TaskIdentity message from the specified reader or buffer.
                 * @param reader Reader or buffer to decode from
                 * @param [length] Message length if known beforehand
                 * @returns {doramcap.telemetry.v1.TaskIdentity & doramcap.telemetry.v1.TaskIdentity.$Shape} TaskIdentity
                 * @throws {Error} If the payload is not a reader or valid buffer
                 * @throws {$protobuf.util.ProtocolError} If required fields are missing
                 */
                static decode(reader: ($protobuf.Reader|Uint8Array), length?: number): doramcap.telemetry.v1.TaskIdentity & doramcap.telemetry.v1.TaskIdentity.$Shape;

                /**
                 * Creates a TaskIdentity message from a plain object. Also converts values to their respective internal types.
                 * @param object Plain object
                 * @returns TaskIdentity
                 */
                static fromObject(object: { [k: string]: any }): doramcap.telemetry.v1.TaskIdentity;

                /**
                 * Creates a plain object from a TaskIdentity message. Also converts values to other types if specified.
                 * @param message TaskIdentity
                 * @param [options] Conversion options
                 * @returns Plain object
                 */
                static toObject(message: doramcap.telemetry.v1.TaskIdentity, options?: $protobuf.IConversionOptions): { [k: string]: any };

                /**
                 * Converts this TaskIdentity to JSON.
                 * @returns JSON object
                 */
                toJSON(): { [k: string]: any };

                /**
                 * Gets the type url for TaskIdentity
                 * @param [prefix] Custom type url prefix, defaults to `"type.googleapis.com"`
                 * @returns The type url
                 */
                static getTypeUrl(prefix?: string): string;
            }

            namespace TaskIdentity {

                /** Properties of a TaskIdentity. */
                interface $Properties {

                    /** TaskIdentity startBoottimeNs */
                    startBoottimeNs?: (number|Long|null);

                    /** TaskIdentity cgroupId */
                    cgroupId?: (number|Long|null);

                    /** TaskIdentity tid */
                    tid?: (number|null);

                    /** TaskIdentity tgid */
                    tgid?: (number|null);

                    /** TaskIdentity cpu */
                    cpu?: (number|null);

                    /** TaskIdentity flags */
                    flags?: (number|null);

                    /** Unknown fields preserved while decoding when enabled */
                    $unknowns?: Uint8Array[];
                }

                /** Shape of a TaskIdentity. */
                type $Shape = doramcap.telemetry.v1.TaskIdentity.$Properties;
            }

            /**
             * Properties of a TaskRecord.
             * @deprecated Use doramcap.telemetry.v1.TaskRecord.$Properties instead.
             */
            interface ITaskRecord extends doramcap.telemetry.v1.TaskRecord.$Properties {
            }

            /** Represents a TaskRecord. */
            class TaskRecord {

                /**
                 * Constructs a new TaskRecord.
                 * @param [properties] Properties to set
                 */
                constructor(properties?: doramcap.telemetry.v1.TaskRecord.$Properties);

                /** Unknown fields preserved while decoding when enabled */
                $unknowns?: Uint8Array[];

                /** TaskRecord identity. */
                identity?: (doramcap.telemetry.v1.TaskIdentity.$Properties|null);

                /** TaskRecord chargedRuntimeNs. */
                chargedRuntimeNs: (number|Long);

                /**
                 * Encodes the specified TaskRecord message. Does not implicitly {@link doramcap.telemetry.v1.TaskRecord.verify|verify} messages.
                 * @param message TaskRecord message or plain object to encode
                 * @param [writer] Writer to encode to
                 * @returns Writer
                 */
                static encode(message: doramcap.telemetry.v1.TaskRecord.$Properties, writer?: $protobuf.Writer): $protobuf.Writer;

                /**
                 * Decodes a TaskRecord message from the specified reader or buffer.
                 * @param reader Reader or buffer to decode from
                 * @param [length] Message length if known beforehand
                 * @returns {doramcap.telemetry.v1.TaskRecord & doramcap.telemetry.v1.TaskRecord.$Shape} TaskRecord
                 * @throws {Error} If the payload is not a reader or valid buffer
                 * @throws {$protobuf.util.ProtocolError} If required fields are missing
                 */
                static decode(reader: ($protobuf.Reader|Uint8Array), length?: number): doramcap.telemetry.v1.TaskRecord & doramcap.telemetry.v1.TaskRecord.$Shape;

                /**
                 * Creates a TaskRecord message from a plain object. Also converts values to their respective internal types.
                 * @param object Plain object
                 * @returns TaskRecord
                 */
                static fromObject(object: { [k: string]: any }): doramcap.telemetry.v1.TaskRecord;

                /**
                 * Creates a plain object from a TaskRecord message. Also converts values to other types if specified.
                 * @param message TaskRecord
                 * @param [options] Conversion options
                 * @returns Plain object
                 */
                static toObject(message: doramcap.telemetry.v1.TaskRecord, options?: $protobuf.IConversionOptions): { [k: string]: any };

                /**
                 * Converts this TaskRecord to JSON.
                 * @returns JSON object
                 */
                toJSON(): { [k: string]: any };

                /**
                 * Gets the type url for TaskRecord
                 * @param [prefix] Custom type url prefix, defaults to `"type.googleapis.com"`
                 * @returns The type url
                 */
                static getTypeUrl(prefix?: string): string;
            }

            namespace TaskRecord {

                /** Properties of a TaskRecord. */
                interface $Properties {

                    /** TaskRecord identity */
                    identity?: (doramcap.telemetry.v1.TaskIdentity.$Properties|null);

                    /** TaskRecord chargedRuntimeNs */
                    chargedRuntimeNs?: (number|Long|null);

                    /** Unknown fields preserved while decoding when enabled */
                    $unknowns?: Uint8Array[];
                }

                /** Shape of a TaskRecord. */
                type $Shape = doramcap.telemetry.v1.TaskRecord.$Properties;
            }

            /**
             * Properties of a TaskLosses.
             * @deprecated Use doramcap.telemetry.v1.TaskLosses.$Properties instead.
             */
            interface ITaskLosses extends doramcap.telemetry.v1.TaskLosses.$Properties {
            }

            /** Represents a TaskLosses. */
            class TaskLosses {

                /**
                 * Constructs a new TaskLosses.
                 * @param [properties] Properties to set
                 */
                constructor(properties?: doramcap.telemetry.v1.TaskLosses.$Properties);

                /** Unknown fields preserved while decoding when enabled */
                $unknowns?: Uint8Array[];

                /** TaskLosses droppedEvents. */
                droppedEvents: (number|Long);

                /** TaskLosses droppedRuntimeNs. */
                droppedRuntimeNs: (number|Long);

                /** TaskLosses identityErrors. */
                identityErrors: (number|Long);

                /**
                 * Encodes the specified TaskLosses message. Does not implicitly {@link doramcap.telemetry.v1.TaskLosses.verify|verify} messages.
                 * @param message TaskLosses message or plain object to encode
                 * @param [writer] Writer to encode to
                 * @returns Writer
                 */
                static encode(message: doramcap.telemetry.v1.TaskLosses.$Properties, writer?: $protobuf.Writer): $protobuf.Writer;

                /**
                 * Decodes a TaskLosses message from the specified reader or buffer.
                 * @param reader Reader or buffer to decode from
                 * @param [length] Message length if known beforehand
                 * @returns {doramcap.telemetry.v1.TaskLosses & doramcap.telemetry.v1.TaskLosses.$Shape} TaskLosses
                 * @throws {Error} If the payload is not a reader or valid buffer
                 * @throws {$protobuf.util.ProtocolError} If required fields are missing
                 */
                static decode(reader: ($protobuf.Reader|Uint8Array), length?: number): doramcap.telemetry.v1.TaskLosses & doramcap.telemetry.v1.TaskLosses.$Shape;

                /**
                 * Creates a TaskLosses message from a plain object. Also converts values to their respective internal types.
                 * @param object Plain object
                 * @returns TaskLosses
                 */
                static fromObject(object: { [k: string]: any }): doramcap.telemetry.v1.TaskLosses;

                /**
                 * Creates a plain object from a TaskLosses message. Also converts values to other types if specified.
                 * @param message TaskLosses
                 * @param [options] Conversion options
                 * @returns Plain object
                 */
                static toObject(message: doramcap.telemetry.v1.TaskLosses, options?: $protobuf.IConversionOptions): { [k: string]: any };

                /**
                 * Converts this TaskLosses to JSON.
                 * @returns JSON object
                 */
                toJSON(): { [k: string]: any };

                /**
                 * Gets the type url for TaskLosses
                 * @param [prefix] Custom type url prefix, defaults to `"type.googleapis.com"`
                 * @returns The type url
                 */
                static getTypeUrl(prefix?: string): string;
            }

            namespace TaskLosses {

                /** Properties of a TaskLosses. */
                interface $Properties {

                    /** TaskLosses droppedEvents */
                    droppedEvents?: (number|Long|null);

                    /** TaskLosses droppedRuntimeNs */
                    droppedRuntimeNs?: (number|Long|null);

                    /** TaskLosses identityErrors */
                    identityErrors?: (number|Long|null);

                    /** Unknown fields preserved while decoding when enabled */
                    $unknowns?: Uint8Array[];
                }

                /** Shape of a TaskLosses. */
                type $Shape = doramcap.telemetry.v1.TaskLosses.$Properties;
            }

            /**
             * Properties of a Snapshot.
             * @deprecated Use doramcap.telemetry.v1.Snapshot.$Properties instead.
             */
            interface ISnapshot extends doramcap.telemetry.v1.Snapshot.$Properties {
            }

            /** Represents a Snapshot. */
            class Snapshot {

                /**
                 * Constructs a new Snapshot.
                 * @param [properties] Properties to set
                 */
                constructor(properties?: doramcap.telemetry.v1.Snapshot.$Properties);

                /** Unknown fields preserved while decoding when enabled */
                $unknowns?: Uint8Array[];

                /** Snapshot schemaVersion. */
                schemaVersion: number;

                /** Snapshot sequence. */
                sequence: (number|Long);

                /** Snapshot captureUnixNs. */
                captureUnixNs: (number|Long);

                /** Snapshot monotonicStartNs. */
                monotonicStartNs: (number|Long);

                /** Snapshot monotonicEndNs. */
                monotonicEndNs: (number|Long);

                /** Snapshot droppedBatches. */
                droppedBatches: (number|Long);

                /** Snapshot capacityOverflow. */
                capacityOverflow: (number|Long);

                /** Snapshot records. */
                records: doramcap.telemetry.v1.CpuRecord.$Properties[];

                /** Snapshot taskRecords. */
                taskRecords: doramcap.telemetry.v1.TaskRecord.$Properties[];

                /** Snapshot taskLosses. */
                taskLosses?: (doramcap.telemetry.v1.TaskLosses.$Properties|null);

                /**
                 * Encodes the specified Snapshot message. Does not implicitly {@link doramcap.telemetry.v1.Snapshot.verify|verify} messages.
                 * @param message Snapshot message or plain object to encode
                 * @param [writer] Writer to encode to
                 * @returns Writer
                 */
                static encode(message: doramcap.telemetry.v1.Snapshot.$Properties, writer?: $protobuf.Writer): $protobuf.Writer;

                /**
                 * Decodes a Snapshot message from the specified reader or buffer.
                 * @param reader Reader or buffer to decode from
                 * @param [length] Message length if known beforehand
                 * @returns {doramcap.telemetry.v1.Snapshot & doramcap.telemetry.v1.Snapshot.$Shape} Snapshot
                 * @throws {Error} If the payload is not a reader or valid buffer
                 * @throws {$protobuf.util.ProtocolError} If required fields are missing
                 */
                static decode(reader: ($protobuf.Reader|Uint8Array), length?: number): doramcap.telemetry.v1.Snapshot & doramcap.telemetry.v1.Snapshot.$Shape;

                /**
                 * Creates a Snapshot message from a plain object. Also converts values to their respective internal types.
                 * @param object Plain object
                 * @returns Snapshot
                 */
                static fromObject(object: { [k: string]: any }): doramcap.telemetry.v1.Snapshot;

                /**
                 * Creates a plain object from a Snapshot message. Also converts values to other types if specified.
                 * @param message Snapshot
                 * @param [options] Conversion options
                 * @returns Plain object
                 */
                static toObject(message: doramcap.telemetry.v1.Snapshot, options?: $protobuf.IConversionOptions): { [k: string]: any };

                /**
                 * Converts this Snapshot to JSON.
                 * @returns JSON object
                 */
                toJSON(): { [k: string]: any };

                /**
                 * Gets the type url for Snapshot
                 * @param [prefix] Custom type url prefix, defaults to `"type.googleapis.com"`
                 * @returns The type url
                 */
                static getTypeUrl(prefix?: string): string;
            }

            namespace Snapshot {

                /** Properties of a Snapshot. */
                interface $Properties {

                    /** Snapshot schemaVersion */
                    schemaVersion?: (number|null);

                    /** Snapshot sequence */
                    sequence?: (number|Long|null);

                    /** Snapshot captureUnixNs */
                    captureUnixNs?: (number|Long|null);

                    /** Snapshot monotonicStartNs */
                    monotonicStartNs?: (number|Long|null);

                    /** Snapshot monotonicEndNs */
                    monotonicEndNs?: (number|Long|null);

                    /** Snapshot droppedBatches */
                    droppedBatches?: (number|Long|null);

                    /** Snapshot capacityOverflow */
                    capacityOverflow?: (number|Long|null);

                    /** Snapshot records */
                    records?: (doramcap.telemetry.v1.CpuRecord.$Properties[]|null);

                    /** Snapshot taskRecords */
                    taskRecords?: (doramcap.telemetry.v1.TaskRecord.$Properties[]|null);

                    /** Snapshot taskLosses */
                    taskLosses?: (doramcap.telemetry.v1.TaskLosses.$Properties|null);

                    /** Unknown fields preserved while decoding when enabled */
                    $unknowns?: Uint8Array[];
                }

                /** Shape of a Snapshot. */
                type $Shape = doramcap.telemetry.v1.Snapshot.$Properties;
            }

            /**
             * Properties of a DictionaryEntry.
             * @deprecated Use doramcap.telemetry.v1.DictionaryEntry.$Properties instead.
             */
            interface IDictionaryEntry extends doramcap.telemetry.v1.DictionaryEntry.$Properties {
            }

            /** Represents a DictionaryEntry. */
            class DictionaryEntry {

                /**
                 * Constructs a new DictionaryEntry.
                 * @param [properties] Properties to set
                 */
                constructor(properties?: doramcap.telemetry.v1.DictionaryEntry.$Properties);

                /** Unknown fields preserved while decoding when enabled */
                $unknowns?: Uint8Array[];

                /** DictionaryEntry id. */
                id: (number|Long);

                /** DictionaryEntry parentId. */
                parentId: (number|Long);

                /** DictionaryEntry path. */
                path?: (string|null);

                /**
                 * Encodes the specified DictionaryEntry message. Does not implicitly {@link doramcap.telemetry.v1.DictionaryEntry.verify|verify} messages.
                 * @param message DictionaryEntry message or plain object to encode
                 * @param [writer] Writer to encode to
                 * @returns Writer
                 */
                static encode(message: doramcap.telemetry.v1.DictionaryEntry.$Properties, writer?: $protobuf.Writer): $protobuf.Writer;

                /**
                 * Decodes a DictionaryEntry message from the specified reader or buffer.
                 * @param reader Reader or buffer to decode from
                 * @param [length] Message length if known beforehand
                 * @returns {doramcap.telemetry.v1.DictionaryEntry & doramcap.telemetry.v1.DictionaryEntry.$Shape} DictionaryEntry
                 * @throws {Error} If the payload is not a reader or valid buffer
                 * @throws {$protobuf.util.ProtocolError} If required fields are missing
                 */
                static decode(reader: ($protobuf.Reader|Uint8Array), length?: number): doramcap.telemetry.v1.DictionaryEntry & doramcap.telemetry.v1.DictionaryEntry.$Shape;

                /**
                 * Creates a DictionaryEntry message from a plain object. Also converts values to their respective internal types.
                 * @param object Plain object
                 * @returns DictionaryEntry
                 */
                static fromObject(object: { [k: string]: any }): doramcap.telemetry.v1.DictionaryEntry;

                /**
                 * Creates a plain object from a DictionaryEntry message. Also converts values to other types if specified.
                 * @param message DictionaryEntry
                 * @param [options] Conversion options
                 * @returns Plain object
                 */
                static toObject(message: doramcap.telemetry.v1.DictionaryEntry, options?: $protobuf.IConversionOptions): { [k: string]: any };

                /**
                 * Converts this DictionaryEntry to JSON.
                 * @returns JSON object
                 */
                toJSON(): { [k: string]: any };

                /**
                 * Gets the type url for DictionaryEntry
                 * @param [prefix] Custom type url prefix, defaults to `"type.googleapis.com"`
                 * @returns The type url
                 */
                static getTypeUrl(prefix?: string): string;
            }

            namespace DictionaryEntry {

                /** Properties of a DictionaryEntry. */
                interface $Properties {

                    /** DictionaryEntry id */
                    id?: (number|Long|null);

                    /** DictionaryEntry parentId */
                    parentId?: (number|Long|null);

                    /** DictionaryEntry path */
                    path?: (string|null);

                    /** Unknown fields preserved while decoding when enabled */
                    $unknowns?: Uint8Array[];
                }

                /** Shape of a DictionaryEntry. */
                type $Shape = doramcap.telemetry.v1.DictionaryEntry.$Properties;
            }

            /**
             * Properties of an Ingress.
             * @deprecated Use doramcap.telemetry.v1.Ingress.$Properties instead.
             */
            interface IIngress extends doramcap.telemetry.v1.Ingress.$Properties {
            }

            /** Represents an Ingress. */
            class Ingress {

                /**
                 * Constructs a new Ingress.
                 * @param [properties] Properties to set
                 */
                constructor(properties?: doramcap.telemetry.v1.Ingress.$Properties);

                /** Unknown fields preserved while decoding when enabled */
                $unknowns?: Uint8Array[];

                /** Ingress totalBytes. */
                totalBytes: (number|Long);

                /** Ingress totalMessages. */
                totalMessages: (number|Long);

                /** Ingress bytesPerSecond. */
                bytesPerSecond?: (number|null);

                /** Ingress messagesPerSecond. */
                messagesPerSecond?: (number|null);

                /** Ingress captureToReceiveMs. */
                captureToReceiveMs?: (number|null);

                /** Ingress meanCaptureToReceiveMs. */
                meanCaptureToReceiveMs?: (number|null);

                /** Ingress invalidTimestamps. */
                invalidTimestamps: (number|Long);

                /**
                 * Encodes the specified Ingress message. Does not implicitly {@link doramcap.telemetry.v1.Ingress.verify|verify} messages.
                 * @param message Ingress message or plain object to encode
                 * @param [writer] Writer to encode to
                 * @returns Writer
                 */
                static encode(message: doramcap.telemetry.v1.Ingress.$Properties, writer?: $protobuf.Writer): $protobuf.Writer;

                /**
                 * Decodes an Ingress message from the specified reader or buffer.
                 * @param reader Reader or buffer to decode from
                 * @param [length] Message length if known beforehand
                 * @returns {doramcap.telemetry.v1.Ingress & doramcap.telemetry.v1.Ingress.$Shape} Ingress
                 * @throws {Error} If the payload is not a reader or valid buffer
                 * @throws {$protobuf.util.ProtocolError} If required fields are missing
                 */
                static decode(reader: ($protobuf.Reader|Uint8Array), length?: number): doramcap.telemetry.v1.Ingress & doramcap.telemetry.v1.Ingress.$Shape;

                /**
                 * Creates an Ingress message from a plain object. Also converts values to their respective internal types.
                 * @param object Plain object
                 * @returns Ingress
                 */
                static fromObject(object: { [k: string]: any }): doramcap.telemetry.v1.Ingress;

                /**
                 * Creates a plain object from an Ingress message. Also converts values to other types if specified.
                 * @param message Ingress
                 * @param [options] Conversion options
                 * @returns Plain object
                 */
                static toObject(message: doramcap.telemetry.v1.Ingress, options?: $protobuf.IConversionOptions): { [k: string]: any };

                /**
                 * Converts this Ingress to JSON.
                 * @returns JSON object
                 */
                toJSON(): { [k: string]: any };

                /**
                 * Gets the type url for Ingress
                 * @param [prefix] Custom type url prefix, defaults to `"type.googleapis.com"`
                 * @returns The type url
                 */
                static getTypeUrl(prefix?: string): string;
            }

            namespace Ingress {

                /** Properties of an Ingress. */
                interface $Properties {

                    /** Ingress totalBytes */
                    totalBytes?: (number|Long|null);

                    /** Ingress totalMessages */
                    totalMessages?: (number|Long|null);

                    /** Ingress bytesPerSecond */
                    bytesPerSecond?: (number|null);

                    /** Ingress messagesPerSecond */
                    messagesPerSecond?: (number|null);

                    /** Ingress captureToReceiveMs */
                    captureToReceiveMs?: (number|null);

                    /** Ingress meanCaptureToReceiveMs */
                    meanCaptureToReceiveMs?: (number|null);

                    /** Ingress invalidTimestamps */
                    invalidTimestamps?: (number|Long|null);

                    /** Unknown fields preserved while decoding when enabled */
                    $unknowns?: Uint8Array[];
                }

                /** Shape of an Ingress. */
                type $Shape = doramcap.telemetry.v1.Ingress.$Properties;
            }

            /**
             * Properties of a SnapshotFrame.
             * @deprecated Use doramcap.telemetry.v1.SnapshotFrame.$Properties instead.
             */
            interface ISnapshotFrame extends doramcap.telemetry.v1.SnapshotFrame.$Properties {
            }

            /** Represents a SnapshotFrame. */
            class SnapshotFrame {

                /**
                 * Constructs a new SnapshotFrame.
                 * @param [properties] Properties to set
                 */
                constructor(properties?: doramcap.telemetry.v1.SnapshotFrame.$Properties);

                /** Unknown fields preserved while decoding when enabled */
                $unknowns?: Uint8Array[];

                /** SnapshotFrame version. */
                version: number;

                /** SnapshotFrame snapshot. */
                snapshot?: (doramcap.telemetry.v1.Snapshot.$Properties|null);

                /** SnapshotFrame source. */
                source: string;

                /** SnapshotFrame taskCollectionEnabled. */
                taskCollectionEnabled: boolean;

                /** SnapshotFrame taskCapacity. */
                taskCapacity: (number|Long);

                /** SnapshotFrame sessionId. */
                sessionId: string;

                /** SnapshotFrame dictionaryRevision. */
                dictionaryRevision: (number|Long);

                /** SnapshotFrame dictionaryFull. */
                dictionaryFull: boolean;

                /** SnapshotFrame dictionaryOverflow. */
                dictionaryOverflow: (number|Long);

                /** SnapshotFrame dictionary. */
                dictionary: doramcap.telemetry.v1.DictionaryEntry.$Properties[];

                /** SnapshotFrame viewerIngress. */
                viewerIngress?: (doramcap.telemetry.v1.Ingress.$Properties|null);

                /**
                 * Encodes the specified SnapshotFrame message. Does not implicitly {@link doramcap.telemetry.v1.SnapshotFrame.verify|verify} messages.
                 * @param message SnapshotFrame message or plain object to encode
                 * @param [writer] Writer to encode to
                 * @returns Writer
                 */
                static encode(message: doramcap.telemetry.v1.SnapshotFrame.$Properties, writer?: $protobuf.Writer): $protobuf.Writer;

                /**
                 * Decodes a SnapshotFrame message from the specified reader or buffer.
                 * @param reader Reader or buffer to decode from
                 * @param [length] Message length if known beforehand
                 * @returns {doramcap.telemetry.v1.SnapshotFrame & doramcap.telemetry.v1.SnapshotFrame.$Shape} SnapshotFrame
                 * @throws {Error} If the payload is not a reader or valid buffer
                 * @throws {$protobuf.util.ProtocolError} If required fields are missing
                 */
                static decode(reader: ($protobuf.Reader|Uint8Array), length?: number): doramcap.telemetry.v1.SnapshotFrame & doramcap.telemetry.v1.SnapshotFrame.$Shape;

                /**
                 * Creates a SnapshotFrame message from a plain object. Also converts values to their respective internal types.
                 * @param object Plain object
                 * @returns SnapshotFrame
                 */
                static fromObject(object: { [k: string]: any }): doramcap.telemetry.v1.SnapshotFrame;

                /**
                 * Creates a plain object from a SnapshotFrame message. Also converts values to other types if specified.
                 * @param message SnapshotFrame
                 * @param [options] Conversion options
                 * @returns Plain object
                 */
                static toObject(message: doramcap.telemetry.v1.SnapshotFrame, options?: $protobuf.IConversionOptions): { [k: string]: any };

                /**
                 * Converts this SnapshotFrame to JSON.
                 * @returns JSON object
                 */
                toJSON(): { [k: string]: any };

                /**
                 * Gets the type url for SnapshotFrame
                 * @param [prefix] Custom type url prefix, defaults to `"type.googleapis.com"`
                 * @returns The type url
                 */
                static getTypeUrl(prefix?: string): string;
            }

            namespace SnapshotFrame {

                /** Properties of a SnapshotFrame. */
                interface $Properties {

                    /** SnapshotFrame version */
                    version?: (number|null);

                    /** SnapshotFrame snapshot */
                    snapshot?: (doramcap.telemetry.v1.Snapshot.$Properties|null);

                    /** SnapshotFrame source */
                    source?: (string|null);

                    /** SnapshotFrame taskCollectionEnabled */
                    taskCollectionEnabled?: (boolean|null);

                    /** SnapshotFrame taskCapacity */
                    taskCapacity?: (number|Long|null);

                    /** SnapshotFrame sessionId */
                    sessionId?: (string|null);

                    /** SnapshotFrame dictionaryRevision */
                    dictionaryRevision?: (number|Long|null);

                    /** SnapshotFrame dictionaryFull */
                    dictionaryFull?: (boolean|null);

                    /** SnapshotFrame dictionaryOverflow */
                    dictionaryOverflow?: (number|Long|null);

                    /** SnapshotFrame dictionary */
                    dictionary?: (doramcap.telemetry.v1.DictionaryEntry.$Properties[]|null);

                    /** SnapshotFrame viewerIngress */
                    viewerIngress?: (doramcap.telemetry.v1.Ingress.$Properties|null);

                    /** Unknown fields preserved while decoding when enabled */
                    $unknowns?: Uint8Array[];
                }

                /** Shape of a SnapshotFrame. */
                type $Shape = doramcap.telemetry.v1.SnapshotFrame.$Properties;
            }

            /**
             * Properties of a DictionaryFrame.
             * @deprecated Use doramcap.telemetry.v1.DictionaryFrame.$Properties instead.
             */
            interface IDictionaryFrame extends doramcap.telemetry.v1.DictionaryFrame.$Properties {
            }

            /** Represents a DictionaryFrame. */
            class DictionaryFrame {

                /**
                 * Constructs a new DictionaryFrame.
                 * @param [properties] Properties to set
                 */
                constructor(properties?: doramcap.telemetry.v1.DictionaryFrame.$Properties);

                /** Unknown fields preserved while decoding when enabled */
                $unknowns?: Uint8Array[];

                /** DictionaryFrame version. */
                version: number;

                /** DictionaryFrame sessionId. */
                sessionId: string;

                /** DictionaryFrame revision. */
                revision: (number|Long);

                /** DictionaryFrame overflow. */
                overflow: (number|Long);

                /** DictionaryFrame entries. */
                entries: doramcap.telemetry.v1.DictionaryEntry.$Properties[];

                /** DictionaryFrame captureUnixNs. */
                captureUnixNs: (number|Long);

                /**
                 * Encodes the specified DictionaryFrame message. Does not implicitly {@link doramcap.telemetry.v1.DictionaryFrame.verify|verify} messages.
                 * @param message DictionaryFrame message or plain object to encode
                 * @param [writer] Writer to encode to
                 * @returns Writer
                 */
                static encode(message: doramcap.telemetry.v1.DictionaryFrame.$Properties, writer?: $protobuf.Writer): $protobuf.Writer;

                /**
                 * Decodes a DictionaryFrame message from the specified reader or buffer.
                 * @param reader Reader or buffer to decode from
                 * @param [length] Message length if known beforehand
                 * @returns {doramcap.telemetry.v1.DictionaryFrame & doramcap.telemetry.v1.DictionaryFrame.$Shape} DictionaryFrame
                 * @throws {Error} If the payload is not a reader or valid buffer
                 * @throws {$protobuf.util.ProtocolError} If required fields are missing
                 */
                static decode(reader: ($protobuf.Reader|Uint8Array), length?: number): doramcap.telemetry.v1.DictionaryFrame & doramcap.telemetry.v1.DictionaryFrame.$Shape;

                /**
                 * Creates a DictionaryFrame message from a plain object. Also converts values to their respective internal types.
                 * @param object Plain object
                 * @returns DictionaryFrame
                 */
                static fromObject(object: { [k: string]: any }): doramcap.telemetry.v1.DictionaryFrame;

                /**
                 * Creates a plain object from a DictionaryFrame message. Also converts values to other types if specified.
                 * @param message DictionaryFrame
                 * @param [options] Conversion options
                 * @returns Plain object
                 */
                static toObject(message: doramcap.telemetry.v1.DictionaryFrame, options?: $protobuf.IConversionOptions): { [k: string]: any };

                /**
                 * Converts this DictionaryFrame to JSON.
                 * @returns JSON object
                 */
                toJSON(): { [k: string]: any };

                /**
                 * Gets the type url for DictionaryFrame
                 * @param [prefix] Custom type url prefix, defaults to `"type.googleapis.com"`
                 * @returns The type url
                 */
                static getTypeUrl(prefix?: string): string;
            }

            namespace DictionaryFrame {

                /** Properties of a DictionaryFrame. */
                interface $Properties {

                    /** DictionaryFrame version */
                    version?: (number|null);

                    /** DictionaryFrame sessionId */
                    sessionId?: (string|null);

                    /** DictionaryFrame revision */
                    revision?: (number|Long|null);

                    /** DictionaryFrame overflow */
                    overflow?: (number|Long|null);

                    /** DictionaryFrame entries */
                    entries?: (doramcap.telemetry.v1.DictionaryEntry.$Properties[]|null);

                    /** DictionaryFrame captureUnixNs */
                    captureUnixNs?: (number|Long|null);

                    /** Unknown fields preserved while decoding when enabled */
                    $unknowns?: Uint8Array[];
                }

                /** Shape of a DictionaryFrame. */
                type $Shape = doramcap.telemetry.v1.DictionaryFrame.$Properties;
            }
        }
    }
}
