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
 * Property enumeration shim for fiftyone-ip-intelligence-sys.
 *
 * The set of properties an IP Intelligence data set was initialized with lives
 * in the `available` member of the base data set, several layers of nested
 * structures deep (DataSetIpi -> DataSetIpiBase b -> DataSetBase b). Reaching it
 * from Rust would require mirroring every intermediate C struct exactly, which
 * is fragile across platforms and compiler versions. The helpers below read
 * those private layouts from C, where the headers describe them, and present a
 * flat C ABI to the Rust crate. They use only the public IP Intelligence and
 * common-cxx headers, so they stay correct as the upstream layouts change.
 *
 * This mirrors the equivalent shim in fiftyone-device-detection-sys. The Hash
 * data set nests as DataSetHash -> DataSetHashBase b -> DataSetBase b, so the
 * `dataSet->b.b.available` access path is the same in both engines.
 */

#include "ipi.h"

/*
 * Returns the number of required (available) properties in the data set managed
 * by `manager`, or zero if the data set or its available properties are not
 * present. The data set reference is taken and released around the read.
 */
uint32_t fiftyoneDegreesShimIpiGetRequiredPropertyCount(
    fiftyoneDegreesResourceManager *manager) {
    fiftyoneDegreesDataSetIpi *dataSet =
        fiftyoneDegreesDataSetIpiGet(manager);
    if (dataSet == NULL) {
        return 0;
    }
    uint32_t count = 0;
    if (dataSet->b.b.available != NULL) {
        count = dataSet->b.b.available->count;
    }
    fiftyoneDegreesDataSetIpiRelease(dataSet);
    return count;
}

/*
 * Writes the name of the required property at `requiredPropertyIndex` into
 * `buffer` as a null terminated string and returns the number of characters
 * written, excluding the terminator. Returns zero when the index is out of
 * range, the buffer is too small or no data set is available. The buffer is
 * always null terminated when `length` is at least one.
 */
size_t fiftyoneDegreesShimIpiGetRequiredPropertyName(
    fiftyoneDegreesResourceManager *manager,
    int requiredPropertyIndex,
    char *buffer,
    size_t length) {
    if (buffer != NULL && length > 0) {
        buffer[0] = '\0';
    }

    fiftyoneDegreesDataSetIpi *dataSet =
        fiftyoneDegreesDataSetIpiGet(manager);
    if (dataSet == NULL) {
        return 0;
    }

    size_t written = 0;
    fiftyoneDegreesPropertiesAvailable *available = dataSet->b.b.available;
    if (available != NULL &&
        requiredPropertyIndex >= 0 &&
        (uint32_t)requiredPropertyIndex < (uint32_t)available->count) {
        fiftyoneDegreesString *name =
            fiftyoneDegreesPropertiesGetNameFromRequiredIndex(
                available, requiredPropertyIndex);
        const char *value = FIFTYONE_DEGREES_STRING(name);
        if (value != NULL && buffer != NULL && length > 0) {
            size_t i = 0;
            while (value[i] != '\0' && i < length - 1) {
                buffer[i] = value[i];
                i++;
            }
            buffer[i] = '\0';
            written = i;
        }
    }

    fiftyoneDegreesDataSetIpiRelease(dataSet);
    return written;
}

/*
 * Returns the zero based required property index for the property named
 * `propertyName`, or -1 when the property is not one of the required
 * properties. This index is the value expected by the results getters such as
 * fiftyoneDegreesResultsIpiGetHasValues and the weighted collection getter
 * fiftyoneDegreesResultsIpiGetValuesCollection.
 */
int fiftyoneDegreesShimIpiGetRequiredPropertyIndexFromName(
    fiftyoneDegreesResourceManager *manager,
    const char *propertyName) {
    fiftyoneDegreesDataSetIpi *dataSet =
        fiftyoneDegreesDataSetIpiGet(manager);
    if (dataSet == NULL) {
        return -1;
    }
    int index = -1;
    if (dataSet->b.b.available != NULL) {
        index = fiftyoneDegreesPropertiesGetRequiredPropertyIndexFromName(
            dataSet->b.b.available, propertyName);
    }
    fiftyoneDegreesDataSetIpiRelease(dataSet);
    return index;
}

/*
 * Writes the data set's name into `buffer` as a null terminated string and
 * returns the number of characters written, excluding the terminator. The data
 * set name is the file's tier, for example "Lite", "Enterprise" or "TAC".
 * Returns zero when no data set is available, the name cannot be read, or the
 * buffer is too small. The buffer is always null terminated when `length` is at
 * least one.
 *
 * The name lives in the strings collection at `header.nameOffset`. This mirrors
 * the Hash engine's data set name accessor in fiftyone-device-detection-sys; the
 * IP Intelligence header carries the same `nameOffset` field.
 */
size_t fiftyoneDegreesShimIpiGetDataSetName(
    fiftyoneDegreesResourceManager *manager,
    char *buffer,
    size_t length) {
    if (buffer != NULL && length > 0) {
        buffer[0] = '\0';
    }

    fiftyoneDegreesDataSetIpi *dataSet =
        fiftyoneDegreesDataSetIpiGet(manager);
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
            const char *value = FIFTYONE_DEGREES_STRING(name);
            if (value != NULL && buffer != NULL && length > 0) {
                size_t i = 0;
                while (value[i] != '\0' && i < length - 1) {
                    buffer[i] = value[i];
                    i++;
                }
                buffer[i] = '\0';
                written = i;
            }
        }
        FIFTYONE_DEGREES_COLLECTION_RELEASE(dataSet->strings, &item);
    }

    fiftyoneDegreesDataSetIpiRelease(dataSet);
    return written;
}

/*
 * Writes the data set's published date into `*year`, `*month` and `*day` and
 * returns 1 on success, or 0 when no data set is available. Any of the output
 * pointers may be NULL. The date is read from the data set header, which carries
 * the embedded publish date of the data file.
 */
int fiftyoneDegreesShimIpiGetDataSetPublished(
    fiftyoneDegreesResourceManager *manager,
    int32_t *year,
    int32_t *month,
    int32_t *day) {
    fiftyoneDegreesDataSetIpi *dataSet =
        fiftyoneDegreesDataSetIpiGet(manager);
    if (dataSet == NULL) {
        return 0;
    }
    if (year != NULL) {
        *year = (int32_t)dataSet->header.published.year;
    }
    if (month != NULL) {
        *month = (int32_t)dataSet->header.published.month;
    }
    if (day != NULL) {
        *day = (int32_t)dataSet->header.published.day;
    }
    fiftyoneDegreesDataSetIpiRelease(dataSet);
    return 1;
}
