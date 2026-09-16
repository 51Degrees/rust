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

#define __STDC_FORMAT_MACROS

#include "string.h"
#include "fiftyone.h"
#include <inttypes.h>

#include "collectionKeyTypes.h"

#ifndef FIFTYONE_DEGREES_MEMORY_ONLY


void* fiftyoneDegreesStoredBinaryValueRead(
    const CollectionFile * const file,
    const CollectionKey * const key,
    Data * const data,
    Exception * const exception) {
#   define MAX_INITIAL_BUFFER_LENGTH 8
    if (key->keyType->initialBytesCount > MAX_INITIAL_BUFFER_LENGTH) {
        EXCEPTION_SET(FIFTYONE_DEGREES_STATUS_INSUFFICIENT_CAPACITY);
        return NULL;
    }
    byte initial[MAX_INITIAL_BUFFER_LENGTH];
#   undef MAX_INITIAL_BUFFER_LENGTH

    return CollectionReadFileVariable(
        file,
        data,
        key,
        &initial,
        exception);
}

#endif

const StoredBinaryValue* fiftyoneDegreesStoredBinaryValueGet(
    const fiftyoneDegreesCollection *strings,
    uint32_t offset,
    PropertyValueType storedValueType,
    fiftyoneDegreesCollectionItem *item,
    Exception *exception) {

    const CollectionKey key = {
        offset,
        GetCollectionKeyTypeForStoredValueType(storedValueType, exception),
    };
    if (EXCEPTION_FAILED) {
        return NULL;
    }

    const fiftyoneDegreesStoredBinaryValue * const result = strings->get(
        strings,
        &key,
        item,
        exception);
    return result;
}

static double shortToDouble(const StoredBinaryValue * const value, const double maxAngle) {
    return (value->shortValue * maxAngle) / INT16_MAX;
}
static double toAzimuth(const StoredBinaryValue * const value) {
    return shortToDouble(value, 180);
}
static double toDeclination(const StoredBinaryValue * const value) {
    return shortToDouble(value, 90);
}

int fiftyoneDegreesStoredBinaryValueCompareWithString(
    const StoredBinaryValue * const value,
    const PropertyValueType storedValueType,
    const char * const target,
    StringBuilder * const tempBuilder,
    Exception * const exception) {

    if (storedValueType == FIFTYONE_DEGREES_PROPERTY_VALUE_TYPE_STRING) {
        const int cmpResult = strncmp(
            &value->stringValue.value,
            target,
            value->stringValue.size);
        return cmpResult;
    }
    EXCEPTION_CLEAR;
    const uint8_t decimalPlaces = (
        tempBuilder->length < MAX_DOUBLE_DECIMAL_PLACES
        ? (uint8_t)tempBuilder->length
        : MAX_DOUBLE_DECIMAL_PLACES);
    StringBuilderAddStringValue(
        tempBuilder,
        value,
        storedValueType,
        decimalPlaces,
        exception);
    StringBuilderComplete(tempBuilder);
    const int result = (EXCEPTION_OKAY
        ? strcmp(tempBuilder->ptr, target)
        : -1);
    return result;
}

int fiftyoneDegreesStoredBinaryValueToIntOrDefault(
    const fiftyoneDegreesStoredBinaryValue * const value,
    const fiftyoneDegreesPropertyValueType storedValueType,
    const int defaultValue) {

    switch (storedValueType) {
        case FIFTYONE_DEGREES_PROPERTY_VALUE_TYPE_STRING: {
            return atoi(&value->stringValue.value);
        }
        case FIFTYONE_DEGREES_PROPERTY_VALUE_TYPE_INTEGER: {
            return value->intValue;
        }
        case FIFTYONE_DEGREES_PROPERTY_VALUE_SINGLE_PRECISION_FLOAT: {
            return (int)FLOAT_TO_NATIVE(value->floatValue);
        }
        case FIFTYONE_DEGREES_PROPERTY_VALUE_TYPE_AZIMUTH: {
            return (int)toAzimuth(value);
        }
        case FIFTYONE_DEGREES_PROPERTY_VALUE_TYPE_DECLINATION: {
            return (int)toDeclination(value);
        }
        case FIFTYONE_DEGREES_PROPERTY_VALUE_SINGLE_BYTE: {
            return value->byteValue;
        }
        default: {
            return defaultValue;
        }
    }
}

double fiftyoneDegreesStoredBinaryValueToDoubleOrDefault(
    const fiftyoneDegreesStoredBinaryValue * const value,
    const fiftyoneDegreesPropertyValueType storedValueType,
    const double defaultValue) {

    switch (storedValueType) {
        case FIFTYONE_DEGREES_PROPERTY_VALUE_TYPE_STRING: {
            return strtod(&value->stringValue.value, NULL);
        }
        case FIFTYONE_DEGREES_PROPERTY_VALUE_TYPE_INTEGER: {
            return value->intValue;
        }
        case FIFTYONE_DEGREES_PROPERTY_VALUE_SINGLE_PRECISION_FLOAT: {
            return FLOAT_TO_NATIVE(value->floatValue);
        }
        case FIFTYONE_DEGREES_PROPERTY_VALUE_TYPE_AZIMUTH: {
            return toAzimuth(value);
        }
        case FIFTYONE_DEGREES_PROPERTY_VALUE_TYPE_DECLINATION: {
            return toDeclination(value);
        }
        case FIFTYONE_DEGREES_PROPERTY_VALUE_SINGLE_BYTE: {
            return value->byteValue;
        }
        default: {
            return defaultValue;
        }
    }
}

bool fiftyoneDegreesStoredBinaryValueToBoolOrDefault(
    const fiftyoneDegreesStoredBinaryValue * const value,
    const fiftyoneDegreesPropertyValueType storedValueType,
    const bool defaultValue) {

    switch (storedValueType) {
        case FIFTYONE_DEGREES_PROPERTY_VALUE_TYPE_STRING: {
            if (value->stringValue.size != 5) {
                return false;
            }
            return !strncmp(&value->stringValue.value, "True", 4);
        }
        case FIFTYONE_DEGREES_PROPERTY_VALUE_TYPE_INTEGER: {
            return value->intValue ? true : false;
        }
        case FIFTYONE_DEGREES_PROPERTY_VALUE_SINGLE_PRECISION_FLOAT: {
            return FLOAT_TO_NATIVE(value->floatValue) ? true : false;
        }
        case FIFTYONE_DEGREES_PROPERTY_VALUE_TYPE_AZIMUTH: {
            return toAzimuth(value) ? true : false;
        }
        case FIFTYONE_DEGREES_PROPERTY_VALUE_TYPE_DECLINATION: {
            return toDeclination(value) ? true : false;
        }
        case FIFTYONE_DEGREES_PROPERTY_VALUE_SINGLE_BYTE: {
            return value->byteValue;
        }
        default: {
            return defaultValue;
        }
    }
}
