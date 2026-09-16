/* *********************************************************************
 * This Original Work is copyright of 51 Degrees Mobile Experts Limited.
 * Copyright 2026 51 Degrees Mobile Experts Limited, Davidson House,
 * Forbury Square, Reading, Berkshire, United Kingdom RG1 3EU.
 *
 * This Original Work is licensed under the European Union Public Licence
 * (EUPL) v.1.2 and is subject to its terms as set out below.
 *
 * If a copy of the EUPL was not distributed with this file, You can obtain
 * one at https://opensource.org/licenses/EUPL-1.2.
 *
 * The 'Compatible Licences' set out in the Appendix to the EUPL (as may be
 * amended by the European Commission) shall be deemed incompatible for
 * the purposes of the Work and the provisions of the compatibility
 * clause in Article 5 of the EUPL shall not apply.
 *
 * If using the Work as, or as part of, a network application, by
 * including the attribution notice(s) required under Article 5 of the EUPL
 * in the end user terms of the application under an appropriate heading,
 * such notice(s) shall fulfill the requirements of that article.
 * ********************************************************************* */

#include "wkbtot.h"
#include <math.h>
#include "fiftyone.h"

typedef uint8_t CoordIndexType;
typedef uint8_t DecimalPlacesType;

typedef struct {
    CoordIndexType dimensionsCount;
    const char *tag;
    size_t tagLength;
} CoordMode;


static const CoordMode CoordModes[] = {
    { 2, NULL, 0 },
    { 3, "Z", 1 },
    { 3, "M", 1 },
    { 4, "ZM", 2 },
};


typedef enum {
    FIFTYONE_DEGREES_WKBToT_ByteOrder_XDR = 0, // Big Endian
    FIFTYONE_DEGREES_WKBToT_ByteOrder_NDR = 1, // Little Endian
} ByteOrder;

#define ByteOrder_XDR FIFTYONE_DEGREES_WKBToT_ByteOrder_XDR
#define ByteOrder_NDR FIFTYONE_DEGREES_WKBToT_ByteOrder_NDR

typedef uint16_t (*RawUShortReader)(const byte **wkbBytes);
typedef int16_t (*RawShortReader)(const byte **wkbBytes);
typedef uint32_t (*RawIntReader)(const byte **wkbBytes);
typedef double (*RawDoubleReader)(const byte **wkbBytes);

static uint8_t readUByte(const byte ** const wkbBytes) {
    const uint8_t result = *(uint8_t *)(*wkbBytes);
    *wkbBytes += 1;
    return result;
}
static int8_t readSByte(const byte ** const wkbBytes) {
    const int8_t result = *(int8_t *)(*wkbBytes);
    *wkbBytes += 1;
    return result;
}
static int16_t readShortMatchingByteOrder(const byte ** const wkbBytes) {
    const int16_t result = *(int16_t *)(*wkbBytes);
    *wkbBytes += 2;
    return result;
}
static uint16_t readUShortMatchingByteOrder(const byte ** const wkbBytes) {
    const uint16_t result = *(uint16_t *)(*wkbBytes);
    *wkbBytes += 2;
    return result;
}
static uint32_t readIntMatchingByteOrder(const byte ** const wkbBytes) {
    const uint32_t result = *(uint32_t *)(*wkbBytes);
    *wkbBytes += 4;
    return result;
}
static double readDoubleMatchingByteOrder(const byte ** const wkbBytes) {
    const double result = *(double *)(*wkbBytes);
    *wkbBytes += 8;
    return result;
}

static uint16_t readUShortMismatchingByteOrder(const byte ** const wkbBytes) {
    byte t[2];
    for (short i = 0; i < 2; i++) {
        t[i] = (*wkbBytes)[2 - i];
    }
    *wkbBytes += 2;
    return *(uint16_t *)t;
}
static int16_t readShortMismatchingByteOrder(const byte ** const wkbBytes) {
    byte t[2];
    for (short i = 0; i < 2; i++) {
        t[i] = (*wkbBytes)[2 - i];
    }
    *wkbBytes += 2;
    return *(int16_t *)t;
}
static uint32_t readIntMismatchingByteOrder(const byte ** const wkbBytes) {
    byte t[4];
    for (short i = 0; i < 4; i++) {
        t[i] = (*wkbBytes)[3 - i];
    }
    *wkbBytes += 4;
    return *(uint32_t *)t;
}
static double readDoubleMismatchingByteOrder(const byte ** const wkbBytes) {
    byte t[8];
    for (short i = 0; i < 8; i++) {
        t[i] = (*wkbBytes)[7 - i];
    }
    *wkbBytes += 8;
    return *(double *)t;
}
typedef struct {
    const char *name;
    RawUShortReader readUShort;
    RawShortReader readShort;
    RawIntReader readInt;
    RawDoubleReader readDouble;
} RawValueReader;

static const RawValueReader MATCHING_BYTE_ORDER_RAW_VALUE_READER = {
    "Matching Byte Order RawValueReader",
    readUShortMatchingByteOrder,
    readShortMatchingByteOrder,
    readIntMatchingByteOrder,
    readDoubleMatchingByteOrder,
};
static const RawValueReader MISMATCHING_BYTE_ORDER_RAW_VALUE_READER = {
    "Mismatching Byte Order RawValueReader",
    readUShortMismatchingByteOrder,
    readShortMismatchingByteOrder,
    readIntMismatchingByteOrder,
    readDoubleMismatchingByteOrder,
};

static ByteOrder getMachineByteOrder() {
    byte buffer[4];
    *(uint32_t *)buffer = 1;
    return buffer[0];
}


typedef enum {
    FIFTYONE_DEGREES_WKBToT_INT_PURPOSE_WKB_TYPE = 0,
    FIFTYONE_DEGREES_WKBToT_INT_PURPOSE_LOOP_COUNT = 1,
} IntPurpose;
#define IntPurpose_WkbType FIFTYONE_DEGREES_WKBToT_INT_PURPOSE_WKB_TYPE
#define IntPurpose_LoopCount FIFTYONE_DEGREES_WKBToT_INT_PURPOSE_LOOP_COUNT

struct num_reader_t;
typedef uint32_t (*IntReader)(const RawValueReader *rawReader, const byte **wkbBytes);
typedef double (*DoubleReader)(const RawValueReader *rawReader, const byte **wkbBytes);
typedef struct num_reader_t {
    const char *name;
    IntReader readInt[2]; // by IntPurpose
    DoubleReader readDouble[4]; // by coord index
} NumReader;

static uint32_t readFullInteger(
    const RawValueReader * const rawReader,
    const byte ** const wkbBytes) {
    return rawReader->readInt(wkbBytes);
}
static double readFullDouble(
    const RawValueReader * const rawReader,
    const byte ** const wkbBytes) {
    return rawReader->readDouble(wkbBytes);
}
static const NumReader NUM_READER_STANDARD = {
    "Standard NumReader",
    readFullInteger,
    readFullInteger,
    readFullDouble,
    readFullDouble,
    readFullDouble,
    readFullDouble,
};

static uint32_t readSingleUByte(
    const RawValueReader * const rawReader,
    const byte **wkbBytes) {
#	ifdef _MSC_VER
    UNREFERENCED_PARAMETER(rawReader);
#	endif
    return readUByte(wkbBytes);
}
static uint32_t readUShort(
    const RawValueReader * const rawReader,
    const byte **wkbBytes) {
    return rawReader->readUShort(wkbBytes);
}
static double readShortAzimuth(
    const RawValueReader * const rawReader,
    const byte **wkbBytes) {
    return (rawReader->readShort(wkbBytes) * 180.0) / INT16_MAX;
}
static double readShortDeclination(
    const RawValueReader * const rawReader,
    const byte **wkbBytes) {
    return (rawReader->readShort(wkbBytes) * 90.0) / INT16_MAX;
}
static const NumReader NUM_READER_REDUCED_SHORT = {
    "Short-Reduced NumReader",
    readSingleUByte,
    readUShort,
    readShortAzimuth,
    readShortDeclination,
    readShortAzimuth,
    readShortDeclination,
};

static const NumReader *selectNumReader(const WkbtotReductionMode reductionMode) {
    switch (reductionMode) {
        case FIFTYONE_DEGREES_WKBToT_REDUCTION_NONE:
        default:
            return &NUM_READER_STANDARD;
        case FIFTYONE_DEGREES_WKBToT_REDUCTION_SHORT:
            return &NUM_READER_REDUCED_SHORT;
    }
}

typedef struct {
    StringBuilder * const stringBuilder;
    bool isSeparated;
} OutputState;

typedef struct {
    const byte *binaryBuffer;
    OutputState output;

    CoordMode coordMode;
    ByteOrder wkbByteOrder;
    ByteOrder const machineByteOrder;
    const RawValueReader *rawValueReader;
    const NumReader * const numReader;

    DecimalPlacesType const decimalPlaces;
    Exception * const exception;
} ProcessingContext;

static uint32_t readInt(
    ProcessingContext * const context,
    const IntPurpose purpose) {

    return context->numReader->readInt[purpose](
        context->rawValueReader,
        &context->binaryBuffer);
}

static double readDouble(
    ProcessingContext * const context,
    const CoordIndexType coordIndex) {

    return context->numReader->readDouble[coordIndex](
        context->rawValueReader,
        &context->binaryBuffer);
}

static void writeEmpty(
    ProcessingContext * const context) {

    static const char empty[] = "EMPTY";
    if (!context->output.isSeparated) {
        StringBuilderAddChar(context->output.stringBuilder, ' ');
    }
    StringBuilderAddChars(context->output.stringBuilder, empty, sizeof(empty) - 1);
    context->output.isSeparated = false;
}

static void writeTaggedGeometryName(
    ProcessingContext * const context,
    const char * const geometryName) {

    StringBuilderAddChars(
        context->output.stringBuilder,
        geometryName,
        strlen(geometryName));
    if (context->coordMode.tag) {
        StringBuilderAddChar(context->output.stringBuilder, ' ');
        StringBuilderAddChars(
            context->output.stringBuilder,
            context->coordMode.tag,
            context->coordMode.tagLength);
    }
    context->output.isSeparated = false;
}



typedef void (*LoopVisitor)(
    ProcessingContext * const context);

static void withParenthesesIterate(
    ProcessingContext * const context,
    const LoopVisitor visitor,
    const uint32_t count) {

    Exception * const exception = context->exception;

    StringBuilderAddChar(context->output.stringBuilder, '(');
    context->output.isSeparated = true;
    for (uint32_t i = 0; i < count; i++) {
        if (i) {
            StringBuilderAddChar(context->output.stringBuilder, ',');
            context->output.isSeparated = true;
        }
        visitor(context);
        if (EXCEPTION_FAILED) {
            return;
        }
    }
    StringBuilderAddChar(context->output.stringBuilder, ')');
    context->output.isSeparated = true;
}

static void handlePointSegment(
    ProcessingContext * const context) {

    for (CoordIndexType i = 0; i < context->coordMode.dimensionsCount; i++) {
        if (i) {
            StringBuilderAddChar(context->output.stringBuilder, ' ');
            context->output.isSeparated = true;
        }
        const double nextCoord = readDouble(context, i);
        StringBuilderAddDouble(context->output.stringBuilder, nextCoord, context->decimalPlaces);
        context->output.isSeparated = false;
    }
}

static void handleLoop(
    ProcessingContext * const context,
    const LoopVisitor visitor) {

    const uint32_t count = readInt(context, IntPurpose_LoopCount);
    if (count) {
        withParenthesesIterate(context, visitor, count);
    } else {
        writeEmpty(context);
    }
}

static void handleLinearRing(
    ProcessingContext * const context) {

    handleLoop(
        context, handlePointSegment);
}


typedef struct GeometryParser_t {
    const char * const nameToPrint;
    const bool hasChildCount;
    const struct GeometryParser_t * const childGeometry;
    const LoopVisitor childParser;
} GeometryParser;

static void handleUnknownGeometry(
    ProcessingContext *context);



static const GeometryParser GEOMETRY_GEOMETRY = {
    // ABSTRACT -- ANY GEOMETRY BELOW QUALIFIES
    "GEOMETRY",
    false,
    NULL,
    writeEmpty,
};
static const GeometryParser GEOMETRY_POINT = {
    "POINT",
    false,
    NULL,
    handlePointSegment,
};
static const GeometryParser GEOMETRY_LINESTRING = {
    "LINESTRING",
    true,
    NULL,
    handlePointSegment,
};
static const GeometryParser GEOMETRY_POLYGON = {
    "POLYGON",
    true,
    NULL,
    handleLinearRing,
};
static const GeometryParser GEOMETRY_MULTIPOINT = {
    "MULTIPOINT",
    true,
    &GEOMETRY_POINT,
    NULL,
};
static const GeometryParser GEOMETRY_MULTILINESTRING = {
    "MULTILINESTRING",
    true,
    &GEOMETRY_LINESTRING,
    NULL,
};
static const GeometryParser GEOMETRY_MULTIPOLYGON = {
    "MULTIPOLYGON",
    true,
    &GEOMETRY_POLYGON,
    NULL,
};
static const GeometryParser GEOMETRY_GEOMETRYCOLLECTION = {
    "GEOMETRYCOLLECTION",
    true,
    NULL,
    handleUnknownGeometry,
};
static const GeometryParser GEOMETRY_CIRCULARSTRING = {
    // RESERVED IN STANDARD (OGC 06-103r4) FOR FUTURE USE
    "CIRCULARSTRING",
    false,
    NULL,
    NULL,
};
static const GeometryParser GEOMETRY_COMPOUNDCURVE = {
    // RESERVED IN STANDARD (OGC 06-103r4) FOR FUTURE USE
    "COMPOUNDCURVE",
    false,
    NULL,
    NULL,
};
static const GeometryParser GEOMETRY_CURVEPOLYGON = {
    // RESERVED IN STANDARD (OGC 06-103r4) FOR FUTURE USE
    "CURVEPOLYGON",
    false,
    NULL,
    NULL,
};
static const GeometryParser GEOMETRY_MULTICURVE = {
    // NON-INSTANTIABLE -- SEE `MultiLineString` SUBCLASS
    "MULTICURVE",
    false,
    NULL,
    NULL,
};
static const GeometryParser GEOMETRY_MULTISURFACE = {
    // NON-INSTANTIABLE -- SEE `MultiPolygon` SUBCLASS
    "MULTISURFACE",
    false,
    NULL,
    NULL,
};
static const GeometryParser GEOMETRY_CURVE = {
    // NON-INSTANTIABLE -- SEE `LineString` SUBCLASS.
    // ALSO `LinearRing` and `Line`
    "CURVE",
    false,
    NULL,
    NULL,
};
static const GeometryParser GEOMETRY_SURFACE = {
    // NON-INSTANTIABLE -- SEE `Polygon` AND `PolyhedralSurface` SUBCLASSES.
    "SURFACE",
    false,
    NULL,
    NULL,
};
static const GeometryParser GEOMETRY_POLYHEDRALSURFACE = {
    "POLYHEDRALSURFACE",
    true,
    &GEOMETRY_POLYGON,
    NULL,
};
static const GeometryParser GEOMETRY_TIN = {
    "TIN",
    true,
    &GEOMETRY_POLYGON,
    NULL,
};
static const GeometryParser GEOMETRY_TRIANGLE = {
    "TRIANGLE",
    true,
    NULL,
    handleLinearRing,
};

static const GeometryParser * const GEOMETRIES[] = {
    &GEOMETRY_GEOMETRY,
    &GEOMETRY_POINT,
    &GEOMETRY_LINESTRING,
    &GEOMETRY_POLYGON,
    &GEOMETRY_MULTIPOINT,
    &GEOMETRY_MULTILINESTRING,
    &GEOMETRY_MULTIPOLYGON,
    &GEOMETRY_GEOMETRYCOLLECTION,
    &GEOMETRY_CIRCULARSTRING,
    &GEOMETRY_COMPOUNDCURVE,
    &GEOMETRY_CURVEPOLYGON,
    &GEOMETRY_MULTICURVE,
    &GEOMETRY_MULTISURFACE,
    &GEOMETRY_CURVE,
    &GEOMETRY_SURFACE,
    &GEOMETRY_POLYHEDRALSURFACE,
    &GEOMETRY_TIN,
    &GEOMETRY_TRIANGLE,
};


static void updateWkbByteOrder(
    ProcessingContext * const context) {

    const ByteOrder newByteOrder = *context->binaryBuffer;
    context->binaryBuffer++;

    if (newByteOrder == context->wkbByteOrder) {
        return;
    }
    context->wkbByteOrder = newByteOrder;
    context->rawValueReader = (
        (context->wkbByteOrder == context->machineByteOrder)
        ? &MATCHING_BYTE_ORDER_RAW_VALUE_READER
        : &MISMATCHING_BYTE_ORDER_RAW_VALUE_READER);
}

static void handleKnownGeometry(
    ProcessingContext *context);

static void handleGeometry(
    ProcessingContext * const context,
    const bool typeIsKnown) {

    updateWkbByteOrder(context);

    const uint32_t geometryTypeFull = readInt(context, IntPurpose_WkbType);
    const uint32_t coordType = geometryTypeFull / 1000;
    const uint32_t geometryCode = geometryTypeFull % 1000;

    context->coordMode = CoordModes[coordType];

    static size_t const GeometriesCount =
        sizeof(GEOMETRIES) / sizeof(GEOMETRIES[0]);
    if (geometryCode >= GeometriesCount) {
        Exception * const exception = context->exception;
        EXCEPTION_SET(UNKNOWN_GEOMETRY);
        return;
    }

    const GeometryParser * const parser =
        GEOMETRIES[geometryCode];
    if (!typeIsKnown && parser->nameToPrint) {
        writeTaggedGeometryName(context, parser->nameToPrint);
    }

    const LoopVisitor visitor = (parser->childGeometry
        ? handleKnownGeometry
        : parser->childParser);
    if (!visitor) {
        Exception * const exception = context->exception;
        EXCEPTION_SET(RESERVED_GEOMETRY);
        return;
    }

    if (parser->hasChildCount) {
        handleLoop(context, visitor);
    } else {
        withParenthesesIterate(context, visitor, 1);
    }
}

static void handleUnknownGeometry(
    ProcessingContext * const context) {

    handleGeometry(context, false);
}

static void handleKnownGeometry(
    ProcessingContext * const context) {

    handleGeometry(context, true);
}

static void handleWKBRoot(
    const byte *binaryBuffer,
    const WkbtotReductionMode reductionMode,
    StringBuilder * const stringBuilder,
    DecimalPlacesType const decimalPlaces,
    Exception * const exception) {

    ProcessingContext context = {
        binaryBuffer,
        {
            stringBuilder,
            true,
        },

        CoordModes[0],
        ~*binaryBuffer,
        getMachineByteOrder(),
        NULL,
        selectNumReader(reductionMode),

        decimalPlaces,
        exception,
    };

    handleUnknownGeometry(&context);
}


void fiftyoneDegreesWriteWkbAsWktToStringBuilder(
    unsigned const char * const wellKnownBinary,
    const WkbtotReductionMode reductionMode,
    const DecimalPlacesType decimalPlaces,
    fiftyoneDegreesStringBuilder * const builder,
    fiftyoneDegreesException * const exception) {

    handleWKBRoot(
        wellKnownBinary,
        reductionMode,
        builder,
        decimalPlaces,
        exception);
}

fiftyoneDegreesWkbtotResult fiftyoneDegreesConvertWkbToWkt(
    const byte * const wellKnownBinary,
    const WkbtotReductionMode reductionMode,
    char * const buffer, size_t const length,
    DecimalPlacesType const decimalPlaces,
    Exception * const exception) {

    StringBuilder stringBuilder = { buffer, length };
    StringBuilderInit(&stringBuilder);

    handleWKBRoot(
        wellKnownBinary,
        reductionMode,
        &stringBuilder,
        decimalPlaces,
        exception);

    StringBuilderComplete(&stringBuilder);

    const fiftyoneDegreesWkbtotResult result = {
        stringBuilder.added,
        stringBuilder.full,
    };
    return result;
}
