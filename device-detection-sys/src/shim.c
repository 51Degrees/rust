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

/*
 * Property enumeration shim for fiftyone-device-detection-sys.
 *
 * The set of properties a Hash data set was initialized with lives in the
 * `available` member of the base data set, several layers of nested structures
 * deep. Reaching it from Rust would require mirroring every intermediate C
 * struct exactly, which is fragile across platforms and compiler versions. The
 * helpers below read those private layouts from C, where the headers describe
 * them, and present a flat C ABI to the Rust crate. They use only the public
 * Hash and common-cxx headers, so they stay correct as the upstream layouts
 * change.
 */

#include "hash/hash.h"

/*
 * Copies `value` into `buffer` as a null terminated string, writing at most
 * `length - 1` characters before the terminator, and returns the number of
 * characters written excluding the terminator. Writes nothing and returns zero
 * when `value` or `buffer` is null or `length` is zero.
 */
static size_t copy_bounded(char *buffer, size_t length, const char *value) {
    size_t written = 0;
    if (value != NULL && buffer != NULL && length > 0) {
        size_t i = 0;
        while (value[i] != '\0' && i < length - 1) {
            buffer[i] = value[i];
            i++;
        }
        buffer[i] = '\0';
        written = i;
    }
    return written;
}

/*
 * Returns the number of required (available) properties in the data set managed
 * by `manager`, or zero if the data set or its available properties are not
 * present. The data set reference is taken and released around the read.
 */
uint32_t fiftyoneDegreesShimHashGetRequiredPropertyCount(
    fiftyoneDegreesResourceManager *manager) {
    fiftyoneDegreesDataSetHash *dataSet =
        fiftyoneDegreesDataSetHashGet(manager);
    if (dataSet == NULL) {
        return 0;
    }
    uint32_t count = 0;
    if (dataSet->b.b.available != NULL) {
        count = dataSet->b.b.available->count;
    }
    fiftyoneDegreesDataSetHashRelease(dataSet);
    return count;
}

/*
 * Writes the name of the required property at `requiredPropertyIndex` into
 * `buffer` as a null terminated string and returns the number of characters
 * written, excluding the terminator. Returns zero when the index is out of
 * range, the buffer is too small or no data set is available. The buffer is
 * always null terminated when `length` is at least one.
 */
size_t fiftyoneDegreesShimHashGetRequiredPropertyName(
    fiftyoneDegreesResourceManager *manager,
    int requiredPropertyIndex,
    char *buffer,
    size_t length) {
    if (buffer != NULL && length > 0) {
        buffer[0] = '\0';
    }

    fiftyoneDegreesDataSetHash *dataSet =
        fiftyoneDegreesDataSetHashGet(manager);
    if (dataSet == NULL) {
        return 0;
    }

    size_t written = 0;
    fiftyoneDegreesPropertiesAvailable *available = dataSet->b.b.available;
    if (available != NULL &&
        requiredPropertyIndex >= 0 &&
        (uint32_t)requiredPropertyIndex < available->count) {
        fiftyoneDegreesString *name =
            fiftyoneDegreesPropertiesGetNameFromRequiredIndex(
                available, requiredPropertyIndex);
        const char *value = FIFTYONE_DEGREES_STRING(name);
        written = copy_bounded(buffer, length, value);
    }

    fiftyoneDegreesDataSetHashRelease(dataSet);
    return written;
}

/*
 * Returns the zero based required property index for the property named
 * `propertyName`, or -1 when the property is not one of the required
 * properties. This index is the value expected by the results getters such as
 * fiftyoneDegreesResultsHashGetHasValues.
 */
int fiftyoneDegreesShimHashGetRequiredPropertyIndexFromName(
    fiftyoneDegreesResourceManager *manager,
    const char *propertyName) {
    fiftyoneDegreesDataSetHash *dataSet =
        fiftyoneDegreesDataSetHashGet(manager);
    if (dataSet == NULL) {
        return -1;
    }
    int index = -1;
    if (dataSet->b.b.available != NULL) {
        index = fiftyoneDegreesPropertiesGetRequiredPropertyIndexFromName(
            dataSet->b.b.available, propertyName);
    }
    fiftyoneDegreesDataSetHashRelease(dataSet);
    return index;
}

/*
 * Writes the count of HTTP header evidence keys in the data set to *count and
 * returns a pointer that is non-null on success. The header names themselves
 * are read one at a time with fiftyoneDegreesShimHashGetEvidenceKey to avoid
 * exposing the headers collection layout.
 */
uint32_t fiftyoneDegreesShimHashGetEvidenceKeyCount(
    fiftyoneDegreesResourceManager *manager) {
    fiftyoneDegreesDataSetHash *dataSet =
        fiftyoneDegreesDataSetHashGet(manager);
    if (dataSet == NULL) {
        return 0;
    }
    uint32_t count = 0;
    if (dataSet->b.b.uniqueHeaders != NULL) {
        count = dataSet->b.b.uniqueHeaders->count;
    }
    fiftyoneDegreesDataSetHashRelease(dataSet);
    return count;
}

/*
 * Writes the name of the HTTP header evidence key at `headerIndex` into
 * `buffer` as a null terminated string and returns the number of characters
 * written, excluding the terminator. Returns zero when the index is out of
 * range, the buffer is too small or no data set is available.
 */
size_t fiftyoneDegreesShimHashGetEvidenceKey(
    fiftyoneDegreesResourceManager *manager,
    uint32_t headerIndex,
    char *buffer,
    size_t length) {
    if (buffer != NULL && length > 0) {
        buffer[0] = '\0';
    }

    fiftyoneDegreesDataSetHash *dataSet =
        fiftyoneDegreesDataSetHashGet(manager);
    if (dataSet == NULL) {
        return 0;
    }

    size_t written = 0;
    fiftyoneDegreesHeaders *headers = dataSet->b.b.uniqueHeaders;
    if (headers != NULL && headerIndex < headers->count) {
        const char *value = headers->items[headerIndex].name;
        written = copy_bounded(buffer, length, value);
    }

    fiftyoneDegreesDataSetHashRelease(dataSet);
    return written;
}

/*
 * Writes the data set's name into `buffer` as a null terminated string and
 * returns the number of characters written, excluding the terminator. The name
 * is the data file's tier (for example "Lite", "Enterprise" or "TAC"), read
 * from the data file header's name offset in the strings collection. Returns
 * zero when the name cannot be read or no data set is available. The buffer is
 * always null terminated when `length` is at least one.
 */
size_t fiftyoneDegreesShimHashGetDataSetName(
    fiftyoneDegreesResourceManager *manager,
    char *buffer,
    size_t length) {
    if (buffer != NULL && length > 0) {
        buffer[0] = '\0';
    }

    fiftyoneDegreesDataSetHash *dataSet =
        fiftyoneDegreesDataSetHashGet(manager);
    if (dataSet == NULL) {
        return 0;
    }

    size_t written = 0;
    FIFTYONE_DEGREES_EXCEPTION_CREATE;
    fiftyoneDegreesCollectionItem item;
    fiftyoneDegreesDataReset(&item.data);
    const fiftyoneDegreesString *name = fiftyoneDegreesStringGet(
        dataSet->strings, (uint32_t)dataSet->header.nameOffset, &item, exception);
    if (name != NULL) {
        if (FIFTYONE_DEGREES_EXCEPTION_OKAY) {
            written = copy_bounded(buffer, length, FIFTYONE_DEGREES_STRING(name));
        }
        FIFTYONE_DEGREES_COLLECTION_RELEASE(dataSet->strings, &item);
    }

    fiftyoneDegreesDataSetHashRelease(dataSet);
    return written;
}

/*
 * Reads the match metrics from the primary (first) result into the output
 * parameters and returns 1, or returns 0 when there is no result. The Hash
 * match metrics (the method, difference, drift, iterations and matched-node
 * count) are computed during detection and live on the result, not in the data
 * file's property values, so they are read here directly rather than through
 * the property value reader. `method` receives the fiftyoneDegreesHashMatchMethod
 * enum value (0 None, 1 Performance, 2 Combined, 3 Predictive). Any output
 * pointer may be null to skip that metric.
 */
int fiftyoneDegreesShimHashGetResultMetrics(
    fiftyoneDegreesResultsHash *results,
    int32_t *method,
    int32_t *difference,
    int32_t *drift,
    int32_t *iterations,
    int32_t *matchedNodes) {
    if (results == NULL || results->count == 0) {
        return 0;
    }
    fiftyoneDegreesResultHash *result = &results->items[0];
    if (method != NULL) {
        *method = (int32_t)result->method;
    }
    if (difference != NULL) {
        *difference = result->difference;
    }
    if (drift != NULL) {
        *drift = result->drift;
    }
    if (iterations != NULL) {
        *iterations = result->iterations;
    }
    if (matchedNodes != NULL) {
        *matchedNodes = result->matchedNodes;
    }
    return 1;
}
