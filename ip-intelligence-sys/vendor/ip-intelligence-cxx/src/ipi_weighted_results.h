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

#ifndef FIFTYONE_DEGREES_IPI_WEIGHTED_RESULTS_INCLUDED
#define FIFTYONE_DEGREES_IPI_WEIGHTED_RESULTS_INCLUDED

/**
 * @file ipi_weighted_results.h
 * @brief Defines structures and functions for handling weighted values in IP Intelligence results.
 * 
 * This file provides the data structures and functions needed to work with weighted values
 * of different types (int, double, bool, byte, string) in the IP Intelligence system.
 * Weighted values include both the actual value and a weighting that indicates the
 * confidence or importance of that value.
 */

#include "ipi.h"
#include "common-cxx/bool.h"
#include "common-cxx/data.h"

/**
 * @brief Header structure for all weighted value types.
 * 
 * This structure serves as the common header for all weighted value types,
 * containing the value type, property index, and raw weighting information.
 */
typedef struct fiftyone_degrees_weighted_value_header_t {
 fiftyoneDegreesPropertyValueType valueType; /**< The type of the property value */
 int requiredPropertyIndex;                  /**< Index of the required property */
 uint32_t rawWeighting;                      /**< Raw confidence weighting value */
} fiftyoneDegreesWeightedValueHeader;


/**
 * @brief Structure for weighted integer values.
 * 
 * Contains a weighted value header and an integer value.
 */
typedef struct fiftyone_degrees_weighted_int_t {
 fiftyoneDegreesWeightedValueHeader header; /**< Common header for all weighted values */
 int32_t value;                             /**< The integer value */
} fiftyoneDegreesWeightedInt;

/**
 * @brief Structure for weighted double values.
 * 
 * Contains a weighted value header and a double value.
 */
typedef struct fiftyone_degrees_weighted_double_t {
 fiftyoneDegreesWeightedValueHeader header; /**< Common header for all weighted values */
 double value;                              /**< The double value */
} fiftyoneDegreesWeightedDouble;

/**
 * @brief Structure for weighted boolean values.
 * 
 * Contains a weighted value header and a boolean value.
 */
typedef struct fiftyone_degrees_weighted_bool_t {
 fiftyoneDegreesWeightedValueHeader header; /**< Common header for all weighted values */
 bool value;                                /**< The boolean value */
} fiftyoneDegreesWeightedBool;

/**
 * @brief Structure for weighted byte values.
 * 
 * Contains a weighted value header and a byte value.
 */
typedef struct fiftyone_degrees_weighted_byte_t {
 fiftyoneDegreesWeightedValueHeader header; /**< Common header for all weighted values */
 uint8_t value;                             /**< The byte value */
} fiftyoneDegreesWeightedByte;

/**
 * @brief Structure for weighted string values.
 * 
 * Contains a weighted value header, string data, and a pointer to the string value.
 * The stringData field owns the memory for the value.
 */
typedef struct fiftyone_degrees_weighted_string_t {
 fiftyoneDegreesWeightedValueHeader header; /**< Common header for all weighted values */
 fiftyoneDegreesData stringData;            /**< Data structure that owns the string memory */
 const char *value;                         /**< Pointer to the string value */
} fiftyoneDegreesWeightedString;


/**
 * @brief Collection of weighted values.
 * 
 * This structure holds a collection of weighted values of various types.
 * It manages the memory for both the values themselves and the array of pointers to them.
 */
typedef struct fiftyone_degrees_weighted_values_collection_t {
 fiftyoneDegreesData valuesData;                    /**< Data structure that owns the actual values */
 fiftyoneDegreesData itemsData;                     /**< Data structure that owns the items table of contents */
 fiftyoneDegreesWeightedValueHeader ** items;       /**< Array of pointers to weighted value headers */
 uint32_t itemsCount;                               /**< Number of items in the collection */
} fiftyoneDegreesWeightedValuesCollection;


/**
 * @brief Gets a collection of weighted values from IP Intelligence results.
 * 
 * This function extracts weighted values for specified properties from the results
 * and returns them as a collection.
 * 
 * @param results Pointer to the IP Intelligence results
 * @param requiredPropertyIndexes Array of required property indexes to extract
 * @param requiredPropertyIndexesLength Number of indexes in the requiredPropertyIndexes array
 * @param tempData Temporary data structure for string conversion operations
 * @param exception Pointer to an exception structure for error handling
 * @return A collection of weighted values
 */
EXTERNAL fiftyoneDegreesWeightedValuesCollection fiftyoneDegreesResultsIpiGetValuesCollection(
 fiftyoneDegreesResultsIpi *results,
 const int *requiredPropertyIndexes,
 uint32_t requiredPropertyIndexesLength,
 fiftyoneDegreesData *tempData,
 fiftyoneDegreesException* exception);

/**
 * @brief Releases resources used by a weighted values collection.
 * 
 * This function frees all memory allocated for the collection and its items.
 * 
 * @param collection Pointer to the collection to release
 */
EXTERNAL void fiftyoneDegreesWeightedValuesCollectionRelease(
 fiftyoneDegreesWeightedValuesCollection *collection);

#endif //FIFTYONE_DEGREES_IPI_WEIGHTED_RESULTS_INCLUDED
