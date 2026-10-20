import * as $protobuf from "protobufjs";
import Long = require("long");

/** Namespace doramcap. */
export namespace doramcap {

    /** Namespace storage. */
    namespace storage {

        /** Namespace v1. */
        namespace v1 {

            /**
             * Properties of a Snapshot.
             * @deprecated Use doramcap.storage.v1.Snapshot.$Properties instead.
             */
            interface ISnapshot extends doramcap.storage.v1.Snapshot.$Properties {
            }

            /** Represents a Snapshot. */
            class Snapshot {

                /**
                 * Constructs a new Snapshot.
                 * @param [properties] Properties to set
                 */
                constructor(properties?: doramcap.storage.v1.Snapshot.$Properties);

                /** Unknown fields preserved while decoding when enabled */
                $unknowns?: Uint8Array[];

                /** Snapshot version. */
                version: number;

                /** Snapshot status. */
                status: string;

                /** Snapshot session. */
                session: string;

                /** Snapshot error. */
                error: string;

                /** Snapshot backend. */
                backend: string;

                /** Snapshot direct. */
                direct: boolean;

                /** Snapshot elapsedMs. */
                elapsedMs: (number|Long);

                /** Snapshot cpu. */
                cpu: (number|Long);

                /** Snapshot heartbeats. */
                heartbeats: (number|Long);

                /** Snapshot probes. */
                probes: (number|Long);

                /** Snapshot inputDrops. */
                inputDrops: (number|Long);

                /** Snapshot lastCpu. */
                lastCpu?: (doramcap.storage.v1.LastCpu.$Properties|null);

                /** Snapshot combining. */
                combining?: (doramcap.storage.v1.Combining.$Properties|null);

                /** Snapshot pool. */
                pool?: (doramcap.storage.v1.Pool.$Properties|null);

                /** Snapshot ring. */
                ring?: (doramcap.storage.v1.Ring.$Properties|null);

                /** Snapshot exercises. */
                exercises?: (doramcap.storage.v1.Exercises.$Properties|null);

                /** Snapshot observation. */
                observation?: (doramcap.storage.v1.Observation.$Properties|null);

                /** Snapshot activeWrites. */
                activeWrites: doramcap.storage.v1.ActiveWrite.$Properties[];

                /** Snapshot timing. */
                timing?: (doramcap.storage.v1.Timing.$Properties|null);

                /** Snapshot latency. */
                latency?: (doramcap.storage.v1.Stats.$Properties|null);

                /** Snapshot lostTimingEvents. */
                lostTimingEvents: (number|Long);

                /** Snapshot burstRemaining. */
                burstRemaining: (number|Long);

                /** Snapshot receipts. */
                receipts: doramcap.storage.v1.Receipt.$Properties[];

                /** Snapshot messages. */
                messages: doramcap.storage.v1.Input.$Properties[];

                /** Snapshot events. */
                events: doramcap.storage.v1.Event.$Properties[];

                /** Snapshot file. */
                file?: (doramcap.storage.v1.File.$Properties|null);

                /** Snapshot lastSealed. */
                lastSealed?: (doramcap.storage.v1.File.$Properties|null);

                /**
                 * Encodes the specified Snapshot message. Does not implicitly {@link doramcap.storage.v1.Snapshot.verify|verify} messages.
                 * @param message Snapshot message or plain object to encode
                 * @param [writer] Writer to encode to
                 * @returns Writer
                 */
                static encode(message: doramcap.storage.v1.Snapshot.$Properties, writer?: $protobuf.Writer): $protobuf.Writer;

                /**
                 * Decodes a Snapshot message from the specified reader or buffer.
                 * @param reader Reader or buffer to decode from
                 * @param [length] Message length if known beforehand
                 * @returns {doramcap.storage.v1.Snapshot & doramcap.storage.v1.Snapshot.$Shape} Snapshot
                 * @throws {Error} If the payload is not a reader or valid buffer
                 * @throws {$protobuf.util.ProtocolError} If required fields are missing
                 */
                static decode(reader: ($protobuf.Reader|Uint8Array), length?: number): doramcap.storage.v1.Snapshot & doramcap.storage.v1.Snapshot.$Shape;

                /**
                 * Creates a Snapshot message from a plain object. Also converts values to their respective internal types.
                 * @param object Plain object
                 * @returns Snapshot
                 */
                static fromObject(object: { [k: string]: any }): doramcap.storage.v1.Snapshot;

                /**
                 * Creates a plain object from a Snapshot message. Also converts values to other types if specified.
                 * @param message Snapshot
                 * @param [options] Conversion options
                 * @returns Plain object
                 */
                static toObject(message: doramcap.storage.v1.Snapshot, options?: $protobuf.IConversionOptions): { [k: string]: any };

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

                    /** Snapshot version */
                    version?: (number|null);

                    /** Snapshot status */
                    status?: (string|null);

                    /** Snapshot session */
                    session?: (string|null);

                    /** Snapshot error */
                    error?: (string|null);

                    /** Snapshot backend */
                    backend?: (string|null);

                    /** Snapshot direct */
                    direct?: (boolean|null);

                    /** Snapshot elapsedMs */
                    elapsedMs?: (number|Long|null);

                    /** Snapshot cpu */
                    cpu?: (number|Long|null);

                    /** Snapshot heartbeats */
                    heartbeats?: (number|Long|null);

                    /** Snapshot probes */
                    probes?: (number|Long|null);

                    /** Snapshot inputDrops */
                    inputDrops?: (number|Long|null);

                    /** Snapshot lastCpu */
                    lastCpu?: (doramcap.storage.v1.LastCpu.$Properties|null);

                    /** Snapshot combining */
                    combining?: (doramcap.storage.v1.Combining.$Properties|null);

                    /** Snapshot pool */
                    pool?: (doramcap.storage.v1.Pool.$Properties|null);

                    /** Snapshot ring */
                    ring?: (doramcap.storage.v1.Ring.$Properties|null);

                    /** Snapshot exercises */
                    exercises?: (doramcap.storage.v1.Exercises.$Properties|null);

                    /** Snapshot observation */
                    observation?: (doramcap.storage.v1.Observation.$Properties|null);

                    /** Snapshot activeWrites */
                    activeWrites?: (doramcap.storage.v1.ActiveWrite.$Properties[]|null);

                    /** Snapshot timing */
                    timing?: (doramcap.storage.v1.Timing.$Properties|null);

                    /** Snapshot latency */
                    latency?: (doramcap.storage.v1.Stats.$Properties|null);

                    /** Snapshot lostTimingEvents */
                    lostTimingEvents?: (number|Long|null);

                    /** Snapshot burstRemaining */
                    burstRemaining?: (number|Long|null);

                    /** Snapshot receipts */
                    receipts?: (doramcap.storage.v1.Receipt.$Properties[]|null);

                    /** Snapshot messages */
                    messages?: (doramcap.storage.v1.Input.$Properties[]|null);

                    /** Snapshot events */
                    events?: (doramcap.storage.v1.Event.$Properties[]|null);

                    /** Snapshot file */
                    file?: (doramcap.storage.v1.File.$Properties|null);

                    /** Snapshot lastSealed */
                    lastSealed?: (doramcap.storage.v1.File.$Properties|null);

                    /** Unknown fields preserved while decoding when enabled */
                    $unknowns?: Uint8Array[];
                }

                /** Shape of a Snapshot. */
                type $Shape = doramcap.storage.v1.Snapshot.$Properties;
            }

            /**
             * Properties of a LastCpu.
             * @deprecated Use doramcap.storage.v1.LastCpu.$Properties instead.
             */
            interface ILastCpu extends doramcap.storage.v1.LastCpu.$Properties {
            }

            /** Represents a LastCpu. */
            class LastCpu {

                /**
                 * Constructs a new LastCpu.
                 * @param [properties] Properties to set
                 */
                constructor(properties?: doramcap.storage.v1.LastCpu.$Properties);

                /** Unknown fields preserved while decoding when enabled */
                $unknowns?: Uint8Array[];

                /** LastCpu sequence. */
                sequence: (number|Long);

                /** LastCpu rows. */
                rows: (number|Long);

                /** LastCpu dropped. */
                dropped: (number|Long);

                /** LastCpu source. */
                source: string;

                /**
                 * Encodes the specified LastCpu message. Does not implicitly {@link doramcap.storage.v1.LastCpu.verify|verify} messages.
                 * @param message LastCpu message or plain object to encode
                 * @param [writer] Writer to encode to
                 * @returns Writer
                 */
                static encode(message: doramcap.storage.v1.LastCpu.$Properties, writer?: $protobuf.Writer): $protobuf.Writer;

                /**
                 * Decodes a LastCpu message from the specified reader or buffer.
                 * @param reader Reader or buffer to decode from
                 * @param [length] Message length if known beforehand
                 * @returns {doramcap.storage.v1.LastCpu & doramcap.storage.v1.LastCpu.$Shape} LastCpu
                 * @throws {Error} If the payload is not a reader or valid buffer
                 * @throws {$protobuf.util.ProtocolError} If required fields are missing
                 */
                static decode(reader: ($protobuf.Reader|Uint8Array), length?: number): doramcap.storage.v1.LastCpu & doramcap.storage.v1.LastCpu.$Shape;

                /**
                 * Creates a LastCpu message from a plain object. Also converts values to their respective internal types.
                 * @param object Plain object
                 * @returns LastCpu
                 */
                static fromObject(object: { [k: string]: any }): doramcap.storage.v1.LastCpu;

                /**
                 * Creates a plain object from a LastCpu message. Also converts values to other types if specified.
                 * @param message LastCpu
                 * @param [options] Conversion options
                 * @returns Plain object
                 */
                static toObject(message: doramcap.storage.v1.LastCpu, options?: $protobuf.IConversionOptions): { [k: string]: any };

                /**
                 * Converts this LastCpu to JSON.
                 * @returns JSON object
                 */
                toJSON(): { [k: string]: any };

                /**
                 * Gets the type url for LastCpu
                 * @param [prefix] Custom type url prefix, defaults to `"type.googleapis.com"`
                 * @returns The type url
                 */
                static getTypeUrl(prefix?: string): string;
            }

            namespace LastCpu {

                /** Properties of a LastCpu. */
                interface $Properties {

                    /** LastCpu sequence */
                    sequence?: (number|Long|null);

                    /** LastCpu rows */
                    rows?: (number|Long|null);

                    /** LastCpu dropped */
                    dropped?: (number|Long|null);

                    /** LastCpu source */
                    source?: (string|null);

                    /** Unknown fields preserved while decoding when enabled */
                    $unknowns?: Uint8Array[];
                }

                /** Shape of a LastCpu. */
                type $Shape = doramcap.storage.v1.LastCpu.$Properties;
            }

            /**
             * Properties of a Combining.
             * @deprecated Use doramcap.storage.v1.Combining.$Properties instead.
             */
            interface ICombining extends doramcap.storage.v1.Combining.$Properties {
            }

            /** Represents a Combining. */
            class Combining {

                /**
                 * Constructs a new Combining.
                 * @param [properties] Properties to set
                 */
                constructor(properties?: doramcap.storage.v1.Combining.$Properties);

                /** Unknown fields preserved while decoding when enabled */
                $unknowns?: Uint8Array[];

                /** Combining used. */
                used: (number|Long);

                /** Combining messages. */
                messages: (number|Long);

                /** Combining capacity. */
                capacity: (number|Long);

                /** Combining ageMs. */
                ageMs?: (number|null);

                /** Combining deadlineMs. */
                deadlineMs: (number|Long);

                /**
                 * Encodes the specified Combining message. Does not implicitly {@link doramcap.storage.v1.Combining.verify|verify} messages.
                 * @param message Combining message or plain object to encode
                 * @param [writer] Writer to encode to
                 * @returns Writer
                 */
                static encode(message: doramcap.storage.v1.Combining.$Properties, writer?: $protobuf.Writer): $protobuf.Writer;

                /**
                 * Decodes a Combining message from the specified reader or buffer.
                 * @param reader Reader or buffer to decode from
                 * @param [length] Message length if known beforehand
                 * @returns {doramcap.storage.v1.Combining & doramcap.storage.v1.Combining.$Shape} Combining
                 * @throws {Error} If the payload is not a reader or valid buffer
                 * @throws {$protobuf.util.ProtocolError} If required fields are missing
                 */
                static decode(reader: ($protobuf.Reader|Uint8Array), length?: number): doramcap.storage.v1.Combining & doramcap.storage.v1.Combining.$Shape;

                /**
                 * Creates a Combining message from a plain object. Also converts values to their respective internal types.
                 * @param object Plain object
                 * @returns Combining
                 */
                static fromObject(object: { [k: string]: any }): doramcap.storage.v1.Combining;

                /**
                 * Creates a plain object from a Combining message. Also converts values to other types if specified.
                 * @param message Combining
                 * @param [options] Conversion options
                 * @returns Plain object
                 */
                static toObject(message: doramcap.storage.v1.Combining, options?: $protobuf.IConversionOptions): { [k: string]: any };

                /**
                 * Converts this Combining to JSON.
                 * @returns JSON object
                 */
                toJSON(): { [k: string]: any };

                /**
                 * Gets the type url for Combining
                 * @param [prefix] Custom type url prefix, defaults to `"type.googleapis.com"`
                 * @returns The type url
                 */
                static getTypeUrl(prefix?: string): string;
            }

            namespace Combining {

                /** Properties of a Combining. */
                interface $Properties {

                    /** Combining used */
                    used?: (number|Long|null);

                    /** Combining messages */
                    messages?: (number|Long|null);

                    /** Combining capacity */
                    capacity?: (number|Long|null);

                    /** Combining ageMs */
                    ageMs?: (number|null);

                    /** Combining deadlineMs */
                    deadlineMs?: (number|Long|null);

                    /** Unknown fields preserved while decoding when enabled */
                    $unknowns?: Uint8Array[];
                }

                /** Shape of a Combining. */
                type $Shape = doramcap.storage.v1.Combining.$Properties;
            }

            /**
             * Properties of a Pool.
             * @deprecated Use doramcap.storage.v1.Pool.$Properties instead.
             */
            interface IPool extends doramcap.storage.v1.Pool.$Properties {
            }

            /** Represents a Pool. */
            class Pool {

                /**
                 * Constructs a new Pool.
                 * @param [properties] Properties to set
                 */
                constructor(properties?: doramcap.storage.v1.Pool.$Properties);

                /** Unknown fields preserved while decoding when enabled */
                $unknowns?: Uint8Array[];

                /** Pool free. */
                free: (number|Long);

                /** Pool total. */
                total: (number|Long);

                /** Pool allocatedBytes. */
                allocatedBytes: (number|Long);

                /** Pool freeBytes. */
                freeBytes: (number|Long);

                /** Pool inFlightBytes. */
                inFlightBytes: (number|Long);

                /** Pool registeredBytes. */
                registeredBytes: (number|Long);

                /**
                 * Encodes the specified Pool message. Does not implicitly {@link doramcap.storage.v1.Pool.verify|verify} messages.
                 * @param message Pool message or plain object to encode
                 * @param [writer] Writer to encode to
                 * @returns Writer
                 */
                static encode(message: doramcap.storage.v1.Pool.$Properties, writer?: $protobuf.Writer): $protobuf.Writer;

                /**
                 * Decodes a Pool message from the specified reader or buffer.
                 * @param reader Reader or buffer to decode from
                 * @param [length] Message length if known beforehand
                 * @returns {doramcap.storage.v1.Pool & doramcap.storage.v1.Pool.$Shape} Pool
                 * @throws {Error} If the payload is not a reader or valid buffer
                 * @throws {$protobuf.util.ProtocolError} If required fields are missing
                 */
                static decode(reader: ($protobuf.Reader|Uint8Array), length?: number): doramcap.storage.v1.Pool & doramcap.storage.v1.Pool.$Shape;

                /**
                 * Creates a Pool message from a plain object. Also converts values to their respective internal types.
                 * @param object Plain object
                 * @returns Pool
                 */
                static fromObject(object: { [k: string]: any }): doramcap.storage.v1.Pool;

                /**
                 * Creates a plain object from a Pool message. Also converts values to other types if specified.
                 * @param message Pool
                 * @param [options] Conversion options
                 * @returns Plain object
                 */
                static toObject(message: doramcap.storage.v1.Pool, options?: $protobuf.IConversionOptions): { [k: string]: any };

                /**
                 * Converts this Pool to JSON.
                 * @returns JSON object
                 */
                toJSON(): { [k: string]: any };

                /**
                 * Gets the type url for Pool
                 * @param [prefix] Custom type url prefix, defaults to `"type.googleapis.com"`
                 * @returns The type url
                 */
                static getTypeUrl(prefix?: string): string;
            }

            namespace Pool {

                /** Properties of a Pool. */
                interface $Properties {

                    /** Pool free */
                    free?: (number|Long|null);

                    /** Pool total */
                    total?: (number|Long|null);

                    /** Pool allocatedBytes */
                    allocatedBytes?: (number|Long|null);

                    /** Pool freeBytes */
                    freeBytes?: (number|Long|null);

                    /** Pool inFlightBytes */
                    inFlightBytes?: (number|Long|null);

                    /** Pool registeredBytes */
                    registeredBytes?: (number|Long|null);

                    /** Unknown fields preserved while decoding when enabled */
                    $unknowns?: Uint8Array[];
                }

                /** Shape of a Pool. */
                type $Shape = doramcap.storage.v1.Pool.$Properties;
            }

            /**
             * Properties of a Ring.
             * @deprecated Use doramcap.storage.v1.Ring.$Properties instead.
             */
            interface IRing extends doramcap.storage.v1.Ring.$Properties {
            }

            /** Represents a Ring. */
            class Ring {

                /**
                 * Constructs a new Ring.
                 * @param [properties] Properties to set
                 */
                constructor(properties?: doramcap.storage.v1.Ring.$Properties);

                /** Unknown fields preserved while decoding when enabled */
                $unknowns?: Uint8Array[];

                /** Ring writer. */
                writer: (number|Long);

                /** Ring file. */
                file: (number|Long);

                /** Ring pendingCapacity. */
                pendingCapacity: (number|Long);

                /** Ring outstanding. */
                outstanding: (number|Long);

                /** Ring submitted. */
                submitted: (number|Long);

                /** Ring completions. */
                completions: (number|Long);

                /** Ring accepted. */
                accepted: (number|Long);

                /** Ring completed. */
                completed: (number|Long);

                /** Ring synced. */
                synced: (number|Long);

                /** Ring awaitingSubmission. */
                awaitingSubmission: (number|Long);

                /** Ring awaitingCqe. */
                awaitingCqe: (number|Long);

                /** Ring pendingBytes. */
                pendingBytes: (number|Long);

                /** Ring oldestUs. */
                oldestUs?: (number|Long|null);

                /** Ring registeredRegions. */
                registeredRegions: (number|Long);

                /** Ring registrationUpdates. */
                registrationUpdates: (number|Long);

                /** Ring acceptedWrites. */
                acceptedWrites: (number|Long);

                /** Ring written. */
                written: (number|Long);

                /** Ring failed. */
                failed: (number|Long);

                /** Ring cancelled. */
                cancelled: (number|Long);

                /** Ring rejectedCapacity. */
                rejectedCapacity: (number|Long);

                /** Ring rejectedInvalid. */
                rejectedInvalid: (number|Long);

                /** Ring rejectedClosed. */
                rejectedClosed: (number|Long);

                /** Ring rejectedOther. */
                rejectedOther: (number|Long);

                /** Ring retries. */
                retries: (number|Long);

                /** Ring registrationHits. */
                registrationHits: (number|Long);

                /** Ring registrationMisses. */
                registrationMisses: (number|Long);

                /** Ring registrationFailures. */
                registrationFailures: (number|Long);

                /** Ring syncFailures. */
                syncFailures: (number|Long);

                /** Ring submitFailures. */
                submitFailures: (number|Long);

                /**
                 * Encodes the specified Ring message. Does not implicitly {@link doramcap.storage.v1.Ring.verify|verify} messages.
                 * @param message Ring message or plain object to encode
                 * @param [writer] Writer to encode to
                 * @returns Writer
                 */
                static encode(message: doramcap.storage.v1.Ring.$Properties, writer?: $protobuf.Writer): $protobuf.Writer;

                /**
                 * Decodes a Ring message from the specified reader or buffer.
                 * @param reader Reader or buffer to decode from
                 * @param [length] Message length if known beforehand
                 * @returns {doramcap.storage.v1.Ring & doramcap.storage.v1.Ring.$Shape} Ring
                 * @throws {Error} If the payload is not a reader or valid buffer
                 * @throws {$protobuf.util.ProtocolError} If required fields are missing
                 */
                static decode(reader: ($protobuf.Reader|Uint8Array), length?: number): doramcap.storage.v1.Ring & doramcap.storage.v1.Ring.$Shape;

                /**
                 * Creates a Ring message from a plain object. Also converts values to their respective internal types.
                 * @param object Plain object
                 * @returns Ring
                 */
                static fromObject(object: { [k: string]: any }): doramcap.storage.v1.Ring;

                /**
                 * Creates a plain object from a Ring message. Also converts values to other types if specified.
                 * @param message Ring
                 * @param [options] Conversion options
                 * @returns Plain object
                 */
                static toObject(message: doramcap.storage.v1.Ring, options?: $protobuf.IConversionOptions): { [k: string]: any };

                /**
                 * Converts this Ring to JSON.
                 * @returns JSON object
                 */
                toJSON(): { [k: string]: any };

                /**
                 * Gets the type url for Ring
                 * @param [prefix] Custom type url prefix, defaults to `"type.googleapis.com"`
                 * @returns The type url
                 */
                static getTypeUrl(prefix?: string): string;
            }

            namespace Ring {

                /** Properties of a Ring. */
                interface $Properties {

                    /** Ring writer */
                    writer?: (number|Long|null);

                    /** Ring file */
                    file?: (number|Long|null);

                    /** Ring pendingCapacity */
                    pendingCapacity?: (number|Long|null);

                    /** Ring outstanding */
                    outstanding?: (number|Long|null);

                    /** Ring submitted */
                    submitted?: (number|Long|null);

                    /** Ring completions */
                    completions?: (number|Long|null);

                    /** Ring accepted */
                    accepted?: (number|Long|null);

                    /** Ring completed */
                    completed?: (number|Long|null);

                    /** Ring synced */
                    synced?: (number|Long|null);

                    /** Ring awaitingSubmission */
                    awaitingSubmission?: (number|Long|null);

                    /** Ring awaitingCqe */
                    awaitingCqe?: (number|Long|null);

                    /** Ring pendingBytes */
                    pendingBytes?: (number|Long|null);

                    /** Ring oldestUs */
                    oldestUs?: (number|Long|null);

                    /** Ring registeredRegions */
                    registeredRegions?: (number|Long|null);

                    /** Ring registrationUpdates */
                    registrationUpdates?: (number|Long|null);

                    /** Ring acceptedWrites */
                    acceptedWrites?: (number|Long|null);

                    /** Ring written */
                    written?: (number|Long|null);

                    /** Ring failed */
                    failed?: (number|Long|null);

                    /** Ring cancelled */
                    cancelled?: (number|Long|null);

                    /** Ring rejectedCapacity */
                    rejectedCapacity?: (number|Long|null);

                    /** Ring rejectedInvalid */
                    rejectedInvalid?: (number|Long|null);

                    /** Ring rejectedClosed */
                    rejectedClosed?: (number|Long|null);

                    /** Ring rejectedOther */
                    rejectedOther?: (number|Long|null);

                    /** Ring retries */
                    retries?: (number|Long|null);

                    /** Ring registrationHits */
                    registrationHits?: (number|Long|null);

                    /** Ring registrationMisses */
                    registrationMisses?: (number|Long|null);

                    /** Ring registrationFailures */
                    registrationFailures?: (number|Long|null);

                    /** Ring syncFailures */
                    syncFailures?: (number|Long|null);

                    /** Ring submitFailures */
                    submitFailures?: (number|Long|null);

                    /** Unknown fields preserved while decoding when enabled */
                    $unknowns?: Uint8Array[];
                }

                /** Shape of a Ring. */
                type $Shape = doramcap.storage.v1.Ring.$Properties;
            }

            /**
             * Properties of an Observation.
             * @deprecated Use doramcap.storage.v1.Observation.$Properties instead.
             */
            interface IObservation extends doramcap.storage.v1.Observation.$Properties {
            }

            /** Represents an Observation. */
            class Observation {

                /**
                 * Constructs a new Observation.
                 * @param [properties] Properties to set
                 */
                constructor(properties?: doramcap.storage.v1.Observation.$Properties);

                /** Unknown fields preserved while decoding when enabled */
                $unknowns?: Uint8Array[];

                /** Observation mode. */
                mode: string;

                /** Observation cursor. */
                cursor: (number|Long);

                /** Observation missed. */
                missed: (number|Long);

                /** Observation contentionDrops. */
                contentionDrops: (number|Long);

                /** Observation overwritten. */
                overwritten: (number|Long);

                /** Observation timelineEvicted. */
                timelineEvicted: (number|Long);

                /** Observation capacity. */
                capacity: (number|Long);

                /**
                 * Encodes the specified Observation message. Does not implicitly {@link doramcap.storage.v1.Observation.verify|verify} messages.
                 * @param message Observation message or plain object to encode
                 * @param [writer] Writer to encode to
                 * @returns Writer
                 */
                static encode(message: doramcap.storage.v1.Observation.$Properties, writer?: $protobuf.Writer): $protobuf.Writer;

                /**
                 * Decodes an Observation message from the specified reader or buffer.
                 * @param reader Reader or buffer to decode from
                 * @param [length] Message length if known beforehand
                 * @returns {doramcap.storage.v1.Observation & doramcap.storage.v1.Observation.$Shape} Observation
                 * @throws {Error} If the payload is not a reader or valid buffer
                 * @throws {$protobuf.util.ProtocolError} If required fields are missing
                 */
                static decode(reader: ($protobuf.Reader|Uint8Array), length?: number): doramcap.storage.v1.Observation & doramcap.storage.v1.Observation.$Shape;

                /**
                 * Creates an Observation message from a plain object. Also converts values to their respective internal types.
                 * @param object Plain object
                 * @returns Observation
                 */
                static fromObject(object: { [k: string]: any }): doramcap.storage.v1.Observation;

                /**
                 * Creates a plain object from an Observation message. Also converts values to other types if specified.
                 * @param message Observation
                 * @param [options] Conversion options
                 * @returns Plain object
                 */
                static toObject(message: doramcap.storage.v1.Observation, options?: $protobuf.IConversionOptions): { [k: string]: any };

                /**
                 * Converts this Observation to JSON.
                 * @returns JSON object
                 */
                toJSON(): { [k: string]: any };

                /**
                 * Gets the type url for Observation
                 * @param [prefix] Custom type url prefix, defaults to `"type.googleapis.com"`
                 * @returns The type url
                 */
                static getTypeUrl(prefix?: string): string;
            }

            namespace Observation {

                /** Properties of an Observation. */
                interface $Properties {

                    /** Observation mode */
                    mode?: (string|null);

                    /** Observation cursor */
                    cursor?: (number|Long|null);

                    /** Observation missed */
                    missed?: (number|Long|null);

                    /** Observation contentionDrops */
                    contentionDrops?: (number|Long|null);

                    /** Observation overwritten */
                    overwritten?: (number|Long|null);

                    /** Observation timelineEvicted */
                    timelineEvicted?: (number|Long|null);

                    /** Observation capacity */
                    capacity?: (number|Long|null);

                    /** Unknown fields preserved while decoding when enabled */
                    $unknowns?: Uint8Array[];
                }

                /** Shape of an Observation. */
                type $Shape = doramcap.storage.v1.Observation.$Properties;
            }

            /**
             * Properties of an ActiveWrite.
             * @deprecated Use doramcap.storage.v1.ActiveWrite.$Properties instead.
             */
            interface IActiveWrite extends doramcap.storage.v1.ActiveWrite.$Properties {
            }

            /** Represents an ActiveWrite. */
            class ActiveWrite {

                /**
                 * Constructs a new ActiveWrite.
                 * @param [properties] Properties to set
                 */
                constructor(properties?: doramcap.storage.v1.ActiveWrite.$Properties);

                /** Unknown fields preserved while decoding when enabled */
                $unknowns?: Uint8Array[];

                /** ActiveWrite write. */
                write: (number|Long);

                /** ActiveWrite operation. */
                operation: (number|Long);

                /** ActiveWrite region. */
                region: (number|Long);

                /** ActiveWrite slabUse. */
                slabUse: (number|Long);

                /** ActiveWrite state. */
                state: string;

                /** ActiveWrite offset. */
                offset: (number|Long);

                /** ActiveWrite len. */
                len: (number|Long);

                /** ActiveWrite completed. */
                completed: (number|Long);

                /** ActiveWrite ageUs. */
                ageUs?: (number|Long|null);

                /** ActiveWrite attempts. */
                attempts: (number|Long);

                /** ActiveWrite retries. */
                retries: (number|Long);

                /**
                 * Encodes the specified ActiveWrite message. Does not implicitly {@link doramcap.storage.v1.ActiveWrite.verify|verify} messages.
                 * @param message ActiveWrite message or plain object to encode
                 * @param [writer] Writer to encode to
                 * @returns Writer
                 */
                static encode(message: doramcap.storage.v1.ActiveWrite.$Properties, writer?: $protobuf.Writer): $protobuf.Writer;

                /**
                 * Decodes an ActiveWrite message from the specified reader or buffer.
                 * @param reader Reader or buffer to decode from
                 * @param [length] Message length if known beforehand
                 * @returns {doramcap.storage.v1.ActiveWrite & doramcap.storage.v1.ActiveWrite.$Shape} ActiveWrite
                 * @throws {Error} If the payload is not a reader or valid buffer
                 * @throws {$protobuf.util.ProtocolError} If required fields are missing
                 */
                static decode(reader: ($protobuf.Reader|Uint8Array), length?: number): doramcap.storage.v1.ActiveWrite & doramcap.storage.v1.ActiveWrite.$Shape;

                /**
                 * Creates an ActiveWrite message from a plain object. Also converts values to their respective internal types.
                 * @param object Plain object
                 * @returns ActiveWrite
                 */
                static fromObject(object: { [k: string]: any }): doramcap.storage.v1.ActiveWrite;

                /**
                 * Creates a plain object from an ActiveWrite message. Also converts values to other types if specified.
                 * @param message ActiveWrite
                 * @param [options] Conversion options
                 * @returns Plain object
                 */
                static toObject(message: doramcap.storage.v1.ActiveWrite, options?: $protobuf.IConversionOptions): { [k: string]: any };

                /**
                 * Converts this ActiveWrite to JSON.
                 * @returns JSON object
                 */
                toJSON(): { [k: string]: any };

                /**
                 * Gets the type url for ActiveWrite
                 * @param [prefix] Custom type url prefix, defaults to `"type.googleapis.com"`
                 * @returns The type url
                 */
                static getTypeUrl(prefix?: string): string;
            }

            namespace ActiveWrite {

                /** Properties of an ActiveWrite. */
                interface $Properties {

                    /** ActiveWrite write */
                    write?: (number|Long|null);

                    /** ActiveWrite operation */
                    operation?: (number|Long|null);

                    /** ActiveWrite region */
                    region?: (number|Long|null);

                    /** ActiveWrite slabUse */
                    slabUse?: (number|Long|null);

                    /** ActiveWrite state */
                    state?: (string|null);

                    /** ActiveWrite offset */
                    offset?: (number|Long|null);

                    /** ActiveWrite len */
                    len?: (number|Long|null);

                    /** ActiveWrite completed */
                    completed?: (number|Long|null);

                    /** ActiveWrite ageUs */
                    ageUs?: (number|Long|null);

                    /** ActiveWrite attempts */
                    attempts?: (number|Long|null);

                    /** ActiveWrite retries */
                    retries?: (number|Long|null);

                    /** Unknown fields preserved while decoding when enabled */
                    $unknowns?: Uint8Array[];
                }

                /** Shape of an ActiveWrite. */
                type $Shape = doramcap.storage.v1.ActiveWrite.$Properties;
            }

            /**
             * Properties of a Timing.
             * @deprecated Use doramcap.storage.v1.Timing.$Properties instead.
             */
            interface ITiming extends doramcap.storage.v1.Timing.$Properties {
            }

            /** Represents a Timing. */
            class Timing {

                /**
                 * Constructs a new Timing.
                 * @param [properties] Properties to set
                 */
                constructor(properties?: doramcap.storage.v1.Timing.$Properties);

                /** Unknown fields preserved while decoding when enabled */
                $unknowns?: Uint8Array[];

                /** Timing sample. */
                sample?: (doramcap.storage.v1.Stats.$Properties|null);

                /** Timing arrival. */
                arrival?: (doramcap.storage.v1.Stats.$Properties|null);

                /** Timing gaps. */
                gaps: (number|Long);

                /** Timing intervalMs. */
                intervalMs?: (number|null);

                /**
                 * Encodes the specified Timing message. Does not implicitly {@link doramcap.storage.v1.Timing.verify|verify} messages.
                 * @param message Timing message or plain object to encode
                 * @param [writer] Writer to encode to
                 * @returns Writer
                 */
                static encode(message: doramcap.storage.v1.Timing.$Properties, writer?: $protobuf.Writer): $protobuf.Writer;

                /**
                 * Decodes a Timing message from the specified reader or buffer.
                 * @param reader Reader or buffer to decode from
                 * @param [length] Message length if known beforehand
                 * @returns {doramcap.storage.v1.Timing & doramcap.storage.v1.Timing.$Shape} Timing
                 * @throws {Error} If the payload is not a reader or valid buffer
                 * @throws {$protobuf.util.ProtocolError} If required fields are missing
                 */
                static decode(reader: ($protobuf.Reader|Uint8Array), length?: number): doramcap.storage.v1.Timing & doramcap.storage.v1.Timing.$Shape;

                /**
                 * Creates a Timing message from a plain object. Also converts values to their respective internal types.
                 * @param object Plain object
                 * @returns Timing
                 */
                static fromObject(object: { [k: string]: any }): doramcap.storage.v1.Timing;

                /**
                 * Creates a plain object from a Timing message. Also converts values to other types if specified.
                 * @param message Timing
                 * @param [options] Conversion options
                 * @returns Plain object
                 */
                static toObject(message: doramcap.storage.v1.Timing, options?: $protobuf.IConversionOptions): { [k: string]: any };

                /**
                 * Converts this Timing to JSON.
                 * @returns JSON object
                 */
                toJSON(): { [k: string]: any };

                /**
                 * Gets the type url for Timing
                 * @param [prefix] Custom type url prefix, defaults to `"type.googleapis.com"`
                 * @returns The type url
                 */
                static getTypeUrl(prefix?: string): string;
            }

            namespace Timing {

                /** Properties of a Timing. */
                interface $Properties {

                    /** Timing sample */
                    sample?: (doramcap.storage.v1.Stats.$Properties|null);

                    /** Timing arrival */
                    arrival?: (doramcap.storage.v1.Stats.$Properties|null);

                    /** Timing gaps */
                    gaps?: (number|Long|null);

                    /** Timing intervalMs */
                    intervalMs?: (number|null);

                    /** Unknown fields preserved while decoding when enabled */
                    $unknowns?: Uint8Array[];
                }

                /** Shape of a Timing. */
                type $Shape = doramcap.storage.v1.Timing.$Properties;
            }

            /**
             * Properties of a Stats.
             * @deprecated Use doramcap.storage.v1.Stats.$Properties instead.
             */
            interface IStats extends doramcap.storage.v1.Stats.$Properties {
            }

            /** Represents a Stats. */
            class Stats {

                /**
                 * Constructs a new Stats.
                 * @param [properties] Properties to set
                 */
                constructor(properties?: doramcap.storage.v1.Stats.$Properties);

                /** Unknown fields preserved while decoding when enabled */
                $unknowns?: Uint8Array[];

                /** Stats count. */
                count: (number|Long);

                /** Stats lastMs. */
                lastMs?: (number|null);

                /** Stats p99Ms. */
                p99Ms?: (number|null);

                /** Stats maxMs. */
                maxMs?: (number|null);

                /**
                 * Encodes the specified Stats message. Does not implicitly {@link doramcap.storage.v1.Stats.verify|verify} messages.
                 * @param message Stats message or plain object to encode
                 * @param [writer] Writer to encode to
                 * @returns Writer
                 */
                static encode(message: doramcap.storage.v1.Stats.$Properties, writer?: $protobuf.Writer): $protobuf.Writer;

                /**
                 * Decodes a Stats message from the specified reader or buffer.
                 * @param reader Reader or buffer to decode from
                 * @param [length] Message length if known beforehand
                 * @returns {doramcap.storage.v1.Stats & doramcap.storage.v1.Stats.$Shape} Stats
                 * @throws {Error} If the payload is not a reader or valid buffer
                 * @throws {$protobuf.util.ProtocolError} If required fields are missing
                 */
                static decode(reader: ($protobuf.Reader|Uint8Array), length?: number): doramcap.storage.v1.Stats & doramcap.storage.v1.Stats.$Shape;

                /**
                 * Creates a Stats message from a plain object. Also converts values to their respective internal types.
                 * @param object Plain object
                 * @returns Stats
                 */
                static fromObject(object: { [k: string]: any }): doramcap.storage.v1.Stats;

                /**
                 * Creates a plain object from a Stats message. Also converts values to other types if specified.
                 * @param message Stats
                 * @param [options] Conversion options
                 * @returns Plain object
                 */
                static toObject(message: doramcap.storage.v1.Stats, options?: $protobuf.IConversionOptions): { [k: string]: any };

                /**
                 * Converts this Stats to JSON.
                 * @returns JSON object
                 */
                toJSON(): { [k: string]: any };

                /**
                 * Gets the type url for Stats
                 * @param [prefix] Custom type url prefix, defaults to `"type.googleapis.com"`
                 * @returns The type url
                 */
                static getTypeUrl(prefix?: string): string;
            }

            namespace Stats {

                /** Properties of a Stats. */
                interface $Properties {

                    /** Stats count */
                    count?: (number|Long|null);

                    /** Stats lastMs */
                    lastMs?: (number|null);

                    /** Stats p99Ms */
                    p99Ms?: (number|null);

                    /** Stats maxMs */
                    maxMs?: (number|null);

                    /** Unknown fields preserved while decoding when enabled */
                    $unknowns?: Uint8Array[];
                }

                /** Shape of a Stats. */
                type $Shape = doramcap.storage.v1.Stats.$Properties;
            }

            /**
             * Properties of a Receipt.
             * @deprecated Use doramcap.storage.v1.Receipt.$Properties instead.
             */
            interface IReceipt extends doramcap.storage.v1.Receipt.$Properties {
            }

            /** Represents a Receipt. */
            class Receipt {

                /**
                 * Constructs a new Receipt.
                 * @param [properties] Properties to set
                 */
                constructor(properties?: doramcap.storage.v1.Receipt.$Properties);

                /** Unknown fields preserved while decoding when enabled */
                $unknowns?: Uint8Array[];

                /** Receipt kind. */
                kind: string;

                /** Receipt count. */
                count: (number|Long);

                /** Receipt bytes. */
                bytes: (number|Long);

                /** Receipt first. */
                first?: (number|Long|null);

                /** Receipt last. */
                last?: (number|Long|null);

                /** Receipt encodeMs. */
                encodeMs: number;

                /**
                 * Encodes the specified Receipt message. Does not implicitly {@link doramcap.storage.v1.Receipt.verify|verify} messages.
                 * @param message Receipt message or plain object to encode
                 * @param [writer] Writer to encode to
                 * @returns Writer
                 */
                static encode(message: doramcap.storage.v1.Receipt.$Properties, writer?: $protobuf.Writer): $protobuf.Writer;

                /**
                 * Decodes a Receipt message from the specified reader or buffer.
                 * @param reader Reader or buffer to decode from
                 * @param [length] Message length if known beforehand
                 * @returns {doramcap.storage.v1.Receipt & doramcap.storage.v1.Receipt.$Shape} Receipt
                 * @throws {Error} If the payload is not a reader or valid buffer
                 * @throws {$protobuf.util.ProtocolError} If required fields are missing
                 */
                static decode(reader: ($protobuf.Reader|Uint8Array), length?: number): doramcap.storage.v1.Receipt & doramcap.storage.v1.Receipt.$Shape;

                /**
                 * Creates a Receipt message from a plain object. Also converts values to their respective internal types.
                 * @param object Plain object
                 * @returns Receipt
                 */
                static fromObject(object: { [k: string]: any }): doramcap.storage.v1.Receipt;

                /**
                 * Creates a plain object from a Receipt message. Also converts values to other types if specified.
                 * @param message Receipt
                 * @param [options] Conversion options
                 * @returns Plain object
                 */
                static toObject(message: doramcap.storage.v1.Receipt, options?: $protobuf.IConversionOptions): { [k: string]: any };

                /**
                 * Converts this Receipt to JSON.
                 * @returns JSON object
                 */
                toJSON(): { [k: string]: any };

                /**
                 * Gets the type url for Receipt
                 * @param [prefix] Custom type url prefix, defaults to `"type.googleapis.com"`
                 * @returns The type url
                 */
                static getTypeUrl(prefix?: string): string;
            }

            namespace Receipt {

                /** Properties of a Receipt. */
                interface $Properties {

                    /** Receipt kind */
                    kind?: (string|null);

                    /** Receipt count */
                    count?: (number|Long|null);

                    /** Receipt bytes */
                    bytes?: (number|Long|null);

                    /** Receipt first */
                    first?: (number|Long|null);

                    /** Receipt last */
                    last?: (number|Long|null);

                    /** Receipt encodeMs */
                    encodeMs?: (number|null);

                    /** Unknown fields preserved while decoding when enabled */
                    $unknowns?: Uint8Array[];
                }

                /** Shape of a Receipt. */
                type $Shape = doramcap.storage.v1.Receipt.$Properties;
            }

            /**
             * Properties of an Input.
             * @deprecated Use doramcap.storage.v1.Input.$Properties instead.
             */
            interface IInput extends doramcap.storage.v1.Input.$Properties {
            }

            /** Represents an Input. */
            class Input {

                /**
                 * Constructs a new Input.
                 * @param [properties] Properties to set
                 */
                constructor(properties?: doramcap.storage.v1.Input.$Properties);

                /** Unknown fields preserved while decoding when enabled */
                $unknowns?: Uint8Array[];

                /** Input sequence. */
                sequence: (number|Long);

                /** Input kind. */
                kind: string;

                /** Input bytes. */
                bytes: (number|Long);

                /** Input timeMs. */
                timeMs: (number|Long);

                /**
                 * Encodes the specified Input message. Does not implicitly {@link doramcap.storage.v1.Input.verify|verify} messages.
                 * @param message Input message or plain object to encode
                 * @param [writer] Writer to encode to
                 * @returns Writer
                 */
                static encode(message: doramcap.storage.v1.Input.$Properties, writer?: $protobuf.Writer): $protobuf.Writer;

                /**
                 * Decodes an Input message from the specified reader or buffer.
                 * @param reader Reader or buffer to decode from
                 * @param [length] Message length if known beforehand
                 * @returns {doramcap.storage.v1.Input & doramcap.storage.v1.Input.$Shape} Input
                 * @throws {Error} If the payload is not a reader or valid buffer
                 * @throws {$protobuf.util.ProtocolError} If required fields are missing
                 */
                static decode(reader: ($protobuf.Reader|Uint8Array), length?: number): doramcap.storage.v1.Input & doramcap.storage.v1.Input.$Shape;

                /**
                 * Creates an Input message from a plain object. Also converts values to their respective internal types.
                 * @param object Plain object
                 * @returns Input
                 */
                static fromObject(object: { [k: string]: any }): doramcap.storage.v1.Input;

                /**
                 * Creates a plain object from an Input message. Also converts values to other types if specified.
                 * @param message Input
                 * @param [options] Conversion options
                 * @returns Plain object
                 */
                static toObject(message: doramcap.storage.v1.Input, options?: $protobuf.IConversionOptions): { [k: string]: any };

                /**
                 * Converts this Input to JSON.
                 * @returns JSON object
                 */
                toJSON(): { [k: string]: any };

                /**
                 * Gets the type url for Input
                 * @param [prefix] Custom type url prefix, defaults to `"type.googleapis.com"`
                 * @returns The type url
                 */
                static getTypeUrl(prefix?: string): string;
            }

            namespace Input {

                /** Properties of an Input. */
                interface $Properties {

                    /** Input sequence */
                    sequence?: (number|Long|null);

                    /** Input kind */
                    kind?: (string|null);

                    /** Input bytes */
                    bytes?: (number|Long|null);

                    /** Input timeMs */
                    timeMs?: (number|Long|null);

                    /** Unknown fields preserved while decoding when enabled */
                    $unknowns?: Uint8Array[];
                }

                /** Shape of an Input. */
                type $Shape = doramcap.storage.v1.Input.$Properties;
            }

            /**
             * Properties of an Event.
             * @deprecated Use doramcap.storage.v1.Event.$Properties instead.
             */
            interface IEvent extends doramcap.storage.v1.Event.$Properties {
            }

            /** Represents an Event. */
            class Event {

                /**
                 * Constructs a new Event.
                 * @param [properties] Properties to set
                 */
                constructor(properties?: doramcap.storage.v1.Event.$Properties);

                /** Unknown fields preserved while decoding when enabled */
                $unknowns?: Uint8Array[];

                /** Event id. */
                id: (number|Long);

                /** Event kind. */
                kind: string;

                /** Event us. */
                us: (number|Long);

                /** Event writer. */
                writer: (number|Long);

                /** Event file. */
                file: (number|Long);

                /** Event write. */
                write: (number|Long);

                /** Event operation. */
                operation: (number|Long);

                /** Event region. */
                region: (number|Long);

                /** Event slabUse. */
                slabUse: (number|Long);

                /** Event pool. */
                pool: (number|Long);

                /** Event offset. */
                offset: (number|Long);

                /** Event len. */
                len: (number|Long);

                /** Event result. */
                result?: (number|Long|null);

                /** Event latencyUs. */
                latencyUs?: (number|Long|null);

                /**
                 * Encodes the specified Event message. Does not implicitly {@link doramcap.storage.v1.Event.verify|verify} messages.
                 * @param message Event message or plain object to encode
                 * @param [writer] Writer to encode to
                 * @returns Writer
                 */
                static encode(message: doramcap.storage.v1.Event.$Properties, writer?: $protobuf.Writer): $protobuf.Writer;

                /**
                 * Decodes an Event message from the specified reader or buffer.
                 * @param reader Reader or buffer to decode from
                 * @param [length] Message length if known beforehand
                 * @returns {doramcap.storage.v1.Event & doramcap.storage.v1.Event.$Shape} Event
                 * @throws {Error} If the payload is not a reader or valid buffer
                 * @throws {$protobuf.util.ProtocolError} If required fields are missing
                 */
                static decode(reader: ($protobuf.Reader|Uint8Array), length?: number): doramcap.storage.v1.Event & doramcap.storage.v1.Event.$Shape;

                /**
                 * Creates an Event message from a plain object. Also converts values to their respective internal types.
                 * @param object Plain object
                 * @returns Event
                 */
                static fromObject(object: { [k: string]: any }): doramcap.storage.v1.Event;

                /**
                 * Creates a plain object from an Event message. Also converts values to other types if specified.
                 * @param message Event
                 * @param [options] Conversion options
                 * @returns Plain object
                 */
                static toObject(message: doramcap.storage.v1.Event, options?: $protobuf.IConversionOptions): { [k: string]: any };

                /**
                 * Converts this Event to JSON.
                 * @returns JSON object
                 */
                toJSON(): { [k: string]: any };

                /**
                 * Gets the type url for Event
                 * @param [prefix] Custom type url prefix, defaults to `"type.googleapis.com"`
                 * @returns The type url
                 */
                static getTypeUrl(prefix?: string): string;
            }

            namespace Event {

                /** Properties of an Event. */
                interface $Properties {

                    /** Event id */
                    id?: (number|Long|null);

                    /** Event kind */
                    kind?: (string|null);

                    /** Event us */
                    us?: (number|Long|null);

                    /** Event writer */
                    writer?: (number|Long|null);

                    /** Event file */
                    file?: (number|Long|null);

                    /** Event write */
                    write?: (number|Long|null);

                    /** Event operation */
                    operation?: (number|Long|null);

                    /** Event region */
                    region?: (number|Long|null);

                    /** Event slabUse */
                    slabUse?: (number|Long|null);

                    /** Event pool */
                    pool?: (number|Long|null);

                    /** Event offset */
                    offset?: (number|Long|null);

                    /** Event len */
                    len?: (number|Long|null);

                    /** Event result */
                    result?: (number|Long|null);

                    /** Event latencyUs */
                    latencyUs?: (number|Long|null);

                    /** Unknown fields preserved while decoding when enabled */
                    $unknowns?: Uint8Array[];
                }

                /** Shape of an Event. */
                type $Shape = doramcap.storage.v1.Event.$Properties;
            }

            /**
             * Properties of a File.
             * @deprecated Use doramcap.storage.v1.File.$Properties instead.
             */
            interface IFile extends doramcap.storage.v1.File.$Properties {
            }

            /** Represents a File. */
            class File {

                /**
                 * Constructs a new File.
                 * @param [properties] Properties to set
                 */
                constructor(properties?: doramcap.storage.v1.File.$Properties);

                /** Unknown fields preserved while decoding when enabled */
                $unknowns?: Uint8Array[];

                /** File index. */
                index: (number|Long);

                /** File name. */
                name: string;

                /** File sealed. */
                sealed: boolean;

                /** File size. */
                size: (number|Long);

                /** File parsedThrough. */
                parsedThrough: (number|Long);

                /** File records. */
                records: doramcap.storage.v1.Record.$Properties[];

                /** File tailOffset. */
                tailOffset: (number|Long);

                /** File tail. */
                tail: Uint8Array;

                /**
                 * Encodes the specified File message. Does not implicitly {@link doramcap.storage.v1.File.verify|verify} messages.
                 * @param message File message or plain object to encode
                 * @param [writer] Writer to encode to
                 * @returns Writer
                 */
                static encode(message: doramcap.storage.v1.File.$Properties, writer?: $protobuf.Writer): $protobuf.Writer;

                /**
                 * Decodes a File message from the specified reader or buffer.
                 * @param reader Reader or buffer to decode from
                 * @param [length] Message length if known beforehand
                 * @returns {doramcap.storage.v1.File & doramcap.storage.v1.File.$Shape} File
                 * @throws {Error} If the payload is not a reader or valid buffer
                 * @throws {$protobuf.util.ProtocolError} If required fields are missing
                 */
                static decode(reader: ($protobuf.Reader|Uint8Array), length?: number): doramcap.storage.v1.File & doramcap.storage.v1.File.$Shape;

                /**
                 * Creates a File message from a plain object. Also converts values to their respective internal types.
                 * @param object Plain object
                 * @returns File
                 */
                static fromObject(object: { [k: string]: any }): doramcap.storage.v1.File;

                /**
                 * Creates a plain object from a File message. Also converts values to other types if specified.
                 * @param message File
                 * @param [options] Conversion options
                 * @returns Plain object
                 */
                static toObject(message: doramcap.storage.v1.File, options?: $protobuf.IConversionOptions): { [k: string]: any };

                /**
                 * Converts this File to JSON.
                 * @returns JSON object
                 */
                toJSON(): { [k: string]: any };

                /**
                 * Gets the type url for File
                 * @param [prefix] Custom type url prefix, defaults to `"type.googleapis.com"`
                 * @returns The type url
                 */
                static getTypeUrl(prefix?: string): string;
            }

            namespace File {

                /** Properties of a File. */
                interface $Properties {

                    /** File index */
                    index?: (number|Long|null);

                    /** File name */
                    name?: (string|null);

                    /** File sealed */
                    sealed?: (boolean|null);

                    /** File size */
                    size?: (number|Long|null);

                    /** File parsedThrough */
                    parsedThrough?: (number|Long|null);

                    /** File records */
                    records?: (doramcap.storage.v1.Record.$Properties[]|null);

                    /** File tailOffset */
                    tailOffset?: (number|Long|null);

                    /** File tail */
                    tail?: (Uint8Array|null);

                    /** Unknown fields preserved while decoding when enabled */
                    $unknowns?: Uint8Array[];
                }

                /** Shape of a File. */
                type $Shape = doramcap.storage.v1.File.$Properties;
            }

            /**
             * Properties of a Record.
             * @deprecated Use doramcap.storage.v1.Record.$Properties instead.
             */
            interface IRecord extends doramcap.storage.v1.Record.$Properties {
            }

            /** Represents a Record. */
            class Record {

                /**
                 * Constructs a new Record.
                 * @param [properties] Properties to set
                 */
                constructor(properties?: doramcap.storage.v1.Record.$Properties);

                /** Unknown fields preserved while decoding when enabled */
                $unknowns?: Uint8Array[];

                /** Record offset. */
                offset: (number|Long);

                /** Record len. */
                len: (number|Long);

                /** Record kind. */
                kind: string;

                /** Record channel. */
                channel?: (number|null);

                /** Record sequence. */
                sequence?: (number|null);

                /** Record payload. */
                payload?: (number|Long|null);

                /**
                 * Encodes the specified Record message. Does not implicitly {@link doramcap.storage.v1.Record.verify|verify} messages.
                 * @param message Record message or plain object to encode
                 * @param [writer] Writer to encode to
                 * @returns Writer
                 */
                static encode(message: doramcap.storage.v1.Record.$Properties, writer?: $protobuf.Writer): $protobuf.Writer;

                /**
                 * Decodes a Record message from the specified reader or buffer.
                 * @param reader Reader or buffer to decode from
                 * @param [length] Message length if known beforehand
                 * @returns {doramcap.storage.v1.Record & doramcap.storage.v1.Record.$Shape} Record
                 * @throws {Error} If the payload is not a reader or valid buffer
                 * @throws {$protobuf.util.ProtocolError} If required fields are missing
                 */
                static decode(reader: ($protobuf.Reader|Uint8Array), length?: number): doramcap.storage.v1.Record & doramcap.storage.v1.Record.$Shape;

                /**
                 * Creates a Record message from a plain object. Also converts values to their respective internal types.
                 * @param object Plain object
                 * @returns Record
                 */
                static fromObject(object: { [k: string]: any }): doramcap.storage.v1.Record;

                /**
                 * Creates a plain object from a Record message. Also converts values to other types if specified.
                 * @param message Record
                 * @param [options] Conversion options
                 * @returns Plain object
                 */
                static toObject(message: doramcap.storage.v1.Record, options?: $protobuf.IConversionOptions): { [k: string]: any };

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

                    /** Record offset */
                    offset?: (number|Long|null);

                    /** Record len */
                    len?: (number|Long|null);

                    /** Record kind */
                    kind?: (string|null);

                    /** Record channel */
                    channel?: (number|null);

                    /** Record sequence */
                    sequence?: (number|null);

                    /** Record payload */
                    payload?: (number|Long|null);

                    /** Unknown fields preserved while decoding when enabled */
                    $unknowns?: Uint8Array[];
                }

                /** Shape of a Record. */
                type $Shape = doramcap.storage.v1.Record.$Properties;
            }

            /**
             * Properties of an Exercises.
             * @deprecated Use doramcap.storage.v1.Exercises.$Properties instead.
             */
            interface IExercises extends doramcap.storage.v1.Exercises.$Properties {
            }

            /** Represents an Exercises. */
            class Exercises {

                /**
                 * Constructs a new Exercises.
                 * @param [properties] Properties to set
                 */
                constructor(properties?: doramcap.storage.v1.Exercises.$Properties);

                /** Unknown fields preserved while decoding when enabled */
                $unknowns?: Uint8Array[];

                /** Exercises receipts. */
                receipts: doramcap.storage.v1.PathReceipt.$Properties[];

                /** Exercises heldReaders. */
                heldReaders: (number|Long);

                /** Exercises pool. */
                pool?: (doramcap.storage.v1.SlabPool.$Properties|null);

                /** Exercises maxWriteBytes. */
                maxWriteBytes: (number|Long);

                /**
                 * Encodes the specified Exercises message. Does not implicitly {@link doramcap.storage.v1.Exercises.verify|verify} messages.
                 * @param message Exercises message or plain object to encode
                 * @param [writer] Writer to encode to
                 * @returns Writer
                 */
                static encode(message: doramcap.storage.v1.Exercises.$Properties, writer?: $protobuf.Writer): $protobuf.Writer;

                /**
                 * Decodes an Exercises message from the specified reader or buffer.
                 * @param reader Reader or buffer to decode from
                 * @param [length] Message length if known beforehand
                 * @returns {doramcap.storage.v1.Exercises & doramcap.storage.v1.Exercises.$Shape} Exercises
                 * @throws {Error} If the payload is not a reader or valid buffer
                 * @throws {$protobuf.util.ProtocolError} If required fields are missing
                 */
                static decode(reader: ($protobuf.Reader|Uint8Array), length?: number): doramcap.storage.v1.Exercises & doramcap.storage.v1.Exercises.$Shape;

                /**
                 * Creates an Exercises message from a plain object. Also converts values to their respective internal types.
                 * @param object Plain object
                 * @returns Exercises
                 */
                static fromObject(object: { [k: string]: any }): doramcap.storage.v1.Exercises;

                /**
                 * Creates a plain object from an Exercises message. Also converts values to other types if specified.
                 * @param message Exercises
                 * @param [options] Conversion options
                 * @returns Plain object
                 */
                static toObject(message: doramcap.storage.v1.Exercises, options?: $protobuf.IConversionOptions): { [k: string]: any };

                /**
                 * Converts this Exercises to JSON.
                 * @returns JSON object
                 */
                toJSON(): { [k: string]: any };

                /**
                 * Gets the type url for Exercises
                 * @param [prefix] Custom type url prefix, defaults to `"type.googleapis.com"`
                 * @returns The type url
                 */
                static getTypeUrl(prefix?: string): string;
            }

            namespace Exercises {

                /** Properties of an Exercises. */
                interface $Properties {

                    /** Exercises receipts */
                    receipts?: (doramcap.storage.v1.PathReceipt.$Properties[]|null);

                    /** Exercises heldReaders */
                    heldReaders?: (number|Long|null);

                    /** Exercises pool */
                    pool?: (doramcap.storage.v1.SlabPool.$Properties|null);

                    /** Exercises maxWriteBytes */
                    maxWriteBytes?: (number|Long|null);

                    /** Unknown fields preserved while decoding when enabled */
                    $unknowns?: Uint8Array[];
                }

                /** Shape of an Exercises. */
                type $Shape = doramcap.storage.v1.Exercises.$Properties;
            }

            /**
             * Properties of a SlabPool.
             * @deprecated Use doramcap.storage.v1.SlabPool.$Properties instead.
             */
            interface ISlabPool extends doramcap.storage.v1.SlabPool.$Properties {
            }

            /** Represents a SlabPool. */
            class SlabPool {

                /**
                 * Constructs a new SlabPool.
                 * @param [properties] Properties to set
                 */
                constructor(properties?: doramcap.storage.v1.SlabPool.$Properties);

                /** Unknown fields preserved while decoding when enabled */
                $unknowns?: Uint8Array[];

                /** SlabPool id. */
                id: (number|Long);

                /** SlabPool available. */
                available: (number|Long);

                /** SlabPool capacity. */
                capacity: (number|Long);

                /** SlabPool slabs. */
                slabs: doramcap.storage.v1.Slab.$Properties[];

                /**
                 * Encodes the specified SlabPool message. Does not implicitly {@link doramcap.storage.v1.SlabPool.verify|verify} messages.
                 * @param message SlabPool message or plain object to encode
                 * @param [writer] Writer to encode to
                 * @returns Writer
                 */
                static encode(message: doramcap.storage.v1.SlabPool.$Properties, writer?: $protobuf.Writer): $protobuf.Writer;

                /**
                 * Decodes a SlabPool message from the specified reader or buffer.
                 * @param reader Reader or buffer to decode from
                 * @param [length] Message length if known beforehand
                 * @returns {doramcap.storage.v1.SlabPool & doramcap.storage.v1.SlabPool.$Shape} SlabPool
                 * @throws {Error} If the payload is not a reader or valid buffer
                 * @throws {$protobuf.util.ProtocolError} If required fields are missing
                 */
                static decode(reader: ($protobuf.Reader|Uint8Array), length?: number): doramcap.storage.v1.SlabPool & doramcap.storage.v1.SlabPool.$Shape;

                /**
                 * Creates a SlabPool message from a plain object. Also converts values to their respective internal types.
                 * @param object Plain object
                 * @returns SlabPool
                 */
                static fromObject(object: { [k: string]: any }): doramcap.storage.v1.SlabPool;

                /**
                 * Creates a plain object from a SlabPool message. Also converts values to other types if specified.
                 * @param message SlabPool
                 * @param [options] Conversion options
                 * @returns Plain object
                 */
                static toObject(message: doramcap.storage.v1.SlabPool, options?: $protobuf.IConversionOptions): { [k: string]: any };

                /**
                 * Converts this SlabPool to JSON.
                 * @returns JSON object
                 */
                toJSON(): { [k: string]: any };

                /**
                 * Gets the type url for SlabPool
                 * @param [prefix] Custom type url prefix, defaults to `"type.googleapis.com"`
                 * @returns The type url
                 */
                static getTypeUrl(prefix?: string): string;
            }

            namespace SlabPool {

                /** Properties of a SlabPool. */
                interface $Properties {

                    /** SlabPool id */
                    id?: (number|Long|null);

                    /** SlabPool available */
                    available?: (number|Long|null);

                    /** SlabPool capacity */
                    capacity?: (number|Long|null);

                    /** SlabPool slabs */
                    slabs?: (doramcap.storage.v1.Slab.$Properties[]|null);

                    /** Unknown fields preserved while decoding when enabled */
                    $unknowns?: Uint8Array[];
                }

                /** Shape of a SlabPool. */
                type $Shape = doramcap.storage.v1.SlabPool.$Properties;
            }

            /**
             * Properties of a Slab.
             * @deprecated Use doramcap.storage.v1.Slab.$Properties instead.
             */
            interface ISlab extends doramcap.storage.v1.Slab.$Properties {
            }

            /** Represents a Slab. */
            class Slab {

                /**
                 * Constructs a new Slab.
                 * @param [properties] Properties to set
                 */
                constructor(properties?: doramcap.storage.v1.Slab.$Properties);

                /** Unknown fields preserved while decoding when enabled */
                $unknowns?: Uint8Array[];

                /** Slab region. */
                region: (number|Long);

                /** Slab use. */
                use: (number|Long);

                /** Slab phase. */
                phase: string;

                /** Slab readers. */
                readers: (number|Long);

                /** Slab ioLeases. */
                ioLeases: (number|Long);

                /** Slab retainedAfterIo. */
                retainedAfterIo: boolean;

                /** Slab ageUs. */
                ageUs?: (number|Long|null);

                /**
                 * Encodes the specified Slab message. Does not implicitly {@link doramcap.storage.v1.Slab.verify|verify} messages.
                 * @param message Slab message or plain object to encode
                 * @param [writer] Writer to encode to
                 * @returns Writer
                 */
                static encode(message: doramcap.storage.v1.Slab.$Properties, writer?: $protobuf.Writer): $protobuf.Writer;

                /**
                 * Decodes a Slab message from the specified reader or buffer.
                 * @param reader Reader or buffer to decode from
                 * @param [length] Message length if known beforehand
                 * @returns {doramcap.storage.v1.Slab & doramcap.storage.v1.Slab.$Shape} Slab
                 * @throws {Error} If the payload is not a reader or valid buffer
                 * @throws {$protobuf.util.ProtocolError} If required fields are missing
                 */
                static decode(reader: ($protobuf.Reader|Uint8Array), length?: number): doramcap.storage.v1.Slab & doramcap.storage.v1.Slab.$Shape;

                /**
                 * Creates a Slab message from a plain object. Also converts values to their respective internal types.
                 * @param object Plain object
                 * @returns Slab
                 */
                static fromObject(object: { [k: string]: any }): doramcap.storage.v1.Slab;

                /**
                 * Creates a plain object from a Slab message. Also converts values to other types if specified.
                 * @param message Slab
                 * @param [options] Conversion options
                 * @returns Plain object
                 */
                static toObject(message: doramcap.storage.v1.Slab, options?: $protobuf.IConversionOptions): { [k: string]: any };

                /**
                 * Converts this Slab to JSON.
                 * @returns JSON object
                 */
                toJSON(): { [k: string]: any };

                /**
                 * Gets the type url for Slab
                 * @param [prefix] Custom type url prefix, defaults to `"type.googleapis.com"`
                 * @returns The type url
                 */
                static getTypeUrl(prefix?: string): string;
            }

            namespace Slab {

                /** Properties of a Slab. */
                interface $Properties {

                    /** Slab region */
                    region?: (number|Long|null);

                    /** Slab use */
                    use?: (number|Long|null);

                    /** Slab phase */
                    phase?: (string|null);

                    /** Slab readers */
                    readers?: (number|Long|null);

                    /** Slab ioLeases */
                    ioLeases?: (number|Long|null);

                    /** Slab retainedAfterIo */
                    retainedAfterIo?: (boolean|null);

                    /** Slab ageUs */
                    ageUs?: (number|Long|null);

                    /** Unknown fields preserved while decoding when enabled */
                    $unknowns?: Uint8Array[];
                }

                /** Shape of a Slab. */
                type $Shape = doramcap.storage.v1.Slab.$Properties;
            }

            /**
             * Properties of a PathReceipt.
             * @deprecated Use doramcap.storage.v1.PathReceipt.$Properties instead.
             */
            interface IPathReceipt extends doramcap.storage.v1.PathReceipt.$Properties {
            }

            /** Represents a PathReceipt. */
            class PathReceipt {

                /**
                 * Constructs a new PathReceipt.
                 * @param [properties] Properties to set
                 */
                constructor(properties?: doramcap.storage.v1.PathReceipt.$Properties);

                /** Unknown fields preserved while decoding when enabled */
                $unknowns?: Uint8Array[];

                /** PathReceipt status. */
                status: string;

                /** PathReceipt error. */
                error: string;

                /** PathReceipt id. */
                id: (number|Long);

                /** PathReceipt action. */
                action: string;

                /** PathReceipt registrationUpdates. */
                registrationUpdates: (number|Long);

                /** PathReceipt messages. */
                messages: (number|Long);

                /** PathReceipt retired. */
                retired: (number|Long);

                /** PathReceipt bytes. */
                bytes: (number|Long);

                /** PathReceipt first. */
                first: number;

                /** PathReceipt last. */
                last: number;

                /** PathReceipt write. */
                write?: (number|Long|null);

                /** PathReceipt region. */
                region?: (number|Long|null);

                /** PathReceipt slabUse. */
                slabUse?: (number|Long|null);

                /** PathReceipt offset. */
                offset?: (number|Long|null);

                /** PathReceipt length. */
                length?: (number|Long|null);

                /**
                 * Encodes the specified PathReceipt message. Does not implicitly {@link doramcap.storage.v1.PathReceipt.verify|verify} messages.
                 * @param message PathReceipt message or plain object to encode
                 * @param [writer] Writer to encode to
                 * @returns Writer
                 */
                static encode(message: doramcap.storage.v1.PathReceipt.$Properties, writer?: $protobuf.Writer): $protobuf.Writer;

                /**
                 * Decodes a PathReceipt message from the specified reader or buffer.
                 * @param reader Reader or buffer to decode from
                 * @param [length] Message length if known beforehand
                 * @returns {doramcap.storage.v1.PathReceipt & doramcap.storage.v1.PathReceipt.$Shape} PathReceipt
                 * @throws {Error} If the payload is not a reader or valid buffer
                 * @throws {$protobuf.util.ProtocolError} If required fields are missing
                 */
                static decode(reader: ($protobuf.Reader|Uint8Array), length?: number): doramcap.storage.v1.PathReceipt & doramcap.storage.v1.PathReceipt.$Shape;

                /**
                 * Creates a PathReceipt message from a plain object. Also converts values to their respective internal types.
                 * @param object Plain object
                 * @returns PathReceipt
                 */
                static fromObject(object: { [k: string]: any }): doramcap.storage.v1.PathReceipt;

                /**
                 * Creates a plain object from a PathReceipt message. Also converts values to other types if specified.
                 * @param message PathReceipt
                 * @param [options] Conversion options
                 * @returns Plain object
                 */
                static toObject(message: doramcap.storage.v1.PathReceipt, options?: $protobuf.IConversionOptions): { [k: string]: any };

                /**
                 * Converts this PathReceipt to JSON.
                 * @returns JSON object
                 */
                toJSON(): { [k: string]: any };

                /**
                 * Gets the type url for PathReceipt
                 * @param [prefix] Custom type url prefix, defaults to `"type.googleapis.com"`
                 * @returns The type url
                 */
                static getTypeUrl(prefix?: string): string;
            }

            namespace PathReceipt {

                /** Properties of a PathReceipt. */
                interface $Properties {

                    /** PathReceipt status */
                    status?: (string|null);

                    /** PathReceipt error */
                    error?: (string|null);

                    /** PathReceipt id */
                    id?: (number|Long|null);

                    /** PathReceipt action */
                    action?: (string|null);

                    /** PathReceipt registrationUpdates */
                    registrationUpdates?: (number|Long|null);

                    /** PathReceipt messages */
                    messages?: (number|Long|null);

                    /** PathReceipt retired */
                    retired?: (number|Long|null);

                    /** PathReceipt bytes */
                    bytes?: (number|Long|null);

                    /** PathReceipt first */
                    first?: (number|null);

                    /** PathReceipt last */
                    last?: (number|null);

                    /** PathReceipt write */
                    write?: (number|Long|null);

                    /** PathReceipt region */
                    region?: (number|Long|null);

                    /** PathReceipt slabUse */
                    slabUse?: (number|Long|null);

                    /** PathReceipt offset */
                    offset?: (number|Long|null);

                    /** PathReceipt length */
                    length?: (number|Long|null);

                    /** Unknown fields preserved while decoding when enabled */
                    $unknowns?: Uint8Array[];
                }

                /** Shape of a PathReceipt. */
                type $Shape = doramcap.storage.v1.PathReceipt.$Properties;
            }
        }
    }
}
