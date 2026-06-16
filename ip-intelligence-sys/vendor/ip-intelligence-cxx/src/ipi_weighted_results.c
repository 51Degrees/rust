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

/**
 * @file ipi_weighted_results.c
 * @brief Implementation of weighted values handling for IP Intelligence results.
 * 
 * This file implements the functions and structures defined in ipi_weighted_results.h.
 * It provides functionality for creating, managing, and releasing collections of
 * weighted values of different types (int, double, bool, byte, string).
 */

#include "ipi_weighted_results.h"
#include "fiftyone.h"


/**
 * @brief Function type for initializing a property value.
 * 
 * @param header Pointer to the weighted value header
 * @param converterState Pointer to converter-specific state data
 */
typedef void (*PropValueInitFunc)(
    WeightedValueHeader *header,
    void *converterState);

/**
 * @brief Function type for saving a property value.
 * 
 * @param header Pointer to the weighted value header
 * @param storedBinaryValue Pointer to the stored binary value
 * @param propertyValueType Type of the property value
 * @param converterState Pointer to converter-specific state data
 * @param exception Pointer to an exception structure for error handling
 */
typedef void (*PropValueSaveFunc)(
    WeightedValueHeader *header,
    const StoredBinaryValue *storedBinaryValue,
    PropertyValueType propertyValueType,
    void *converterState,
    Exception *exception);

/**
 * @brief Function type for freeing a property value.
 * 
 * @param header Pointer to the weighted value header
 */
typedef void (*PropValueFreeFunc)(
    WeightedValueHeader *header);

/**
 * @brief Structure defining a converter for property values.
 * 
 * This structure contains all the information needed to convert property values
 * of a specific type, including initialization, saving, and freeing functions.
 */
typedef struct {
    const char * const name;                /**< Name of the converter */
    const PropertyValueType valueType;      /**< Type of property value this converter handles */
    const PropValueInitFunc itemInitFunc;   /**< Function to initialize values */
    const PropValueSaveFunc itemSaveFunc;   /**< Function to save values */
    const PropValueFreeFunc itemFreeFunc;   /**< Function to free values (can be NULL) */
    const size_t itemSize;                  /**< Size of the value structure */
} PropValuesConverter;


/**
 * @brief Structure for a chunk of property values of the same type.
 * 
 * This structure holds a collection of property values for a single property.
 */
typedef struct {
    int requiredPropertyIndex;              /**< Index of the required property */
    Data data;                              /**< Data structure holding the values */
    uint32_t count;                         /**< Number of values in the chunk */
    const PropValuesConverter *converter;   /**< Converter for this value type */
} PropValuesChunk;

/**
 * @brief Structure for a collection of property value chunks.
 * 
 * This structure holds multiple chunks of property values, where each chunk
 * corresponds to a different property.
 */
typedef struct {
    uint32_t count;                         /**< Number of chunks */
    Data data;                              /**< Data structure holding the chunks */
    PropValuesChunk *items;                 /**< Array of chunks */
} PropValues;

/**
 * @brief Initializes a PropValues structure.
 * 
 * Allocates memory for the specified number of property value chunks.
 * 
 * @param values Pointer to the PropValues structure to initialize
 * @param count Number of chunks to allocate
 */
static void PropValuesInit(PropValues * const values, const uint32_t count) {
    values->count = count;
    DataReset(&values->data);
    DataMalloc(&values->data, count * sizeof(PropValuesChunk));
    values->data.used = values->data.allocated;
    values->items = (PropValuesChunk *)values->data.ptr;
    for (uint32_t i = 0, n = values->count; i < n; i++) {
        DataReset(&values->items[i].data);
    }
}

/**
 * @brief Releases resources used by a PropValues structure.
 * 
 * Frees all memory allocated for the PropValues structure and its chunks.
 * 
 * @param values Pointer to the PropValues structure to release
 */
static void PropValuesRelease(PropValues * const values) {
    if (values->items && values->count > 0) {
        for (uint32_t i = 0, n = values->count; i < n; i++) {
            Data * const nextChunkData = &(values->items[i].data);
            if (nextChunkData->allocated) {
                Free(nextChunkData->ptr);
                DataReset(nextChunkData);
            }
        }
    }
    if (values->data.allocated) {
        Free(values->data.ptr);
        DataReset(&values->data);
    }
    values->count = 0;
    values->items = NULL;
}

/**
 * @brief Initializes an integer weighted value.
 * 
 * Sets the integer value from the converter state.
 * 
 * @param header Pointer to the weighted value header
 * @param converterState Pointer to the default integer value
 */
static void InitInt(
    WeightedValueHeader * const header,
    void * const converterState) {
    ((WeightedInt*)header)->value = *(int*)converterState;
}
/**
 * @brief Saves an integer value from a stored binary value.
 * 
 * Converts the stored binary value to an integer and saves it in the weighted value.
 * 
 * @param header Pointer to the weighted value header
 * @param storedBinaryValue Pointer to the stored binary value
 * @param propertyValueType Type of the property value
 * @param converterState Pointer to the default integer value
 * @param exception Pointer to an exception structure (unused)
 */
static void SaveInt(
    WeightedValueHeader * const header,
    const StoredBinaryValue * const storedBinaryValue,
    const PropertyValueType propertyValueType,
    void * const converterState,
    Exception * const exception) {
#	ifdef _MSC_VER
    UNREFERENCED_PARAMETER(exception);
#	endif

    ((WeightedInt*)header)->value = StoredBinaryValueToIntOrDefault(
        storedBinaryValue, propertyValueType, *(int*)converterState);
}

/**
 * @brief Initializes a boolean weighted value.
 * 
 * Sets the boolean value from the converter state.
 * 
 * @param header Pointer to the weighted value header
 * @param converterState Pointer to the default boolean value
 */
static void InitBool(
    WeightedValueHeader * const header,
    void * const converterState) {
    ((WeightedBool*)header)->value = *(bool*)converterState;
}
/**
 * @brief Saves a boolean value from a stored binary value.
 * 
 * Converts the stored binary value to a boolean and saves it in the weighted value.
 * 
 * @param header Pointer to the weighted value header
 * @param storedBinaryValue Pointer to the stored binary value
 * @param propertyValueType Type of the property value
 * @param converterState Pointer to the default boolean value
 * @param exception Pointer to an exception structure (unused)
 */
static void SaveBool(
    WeightedValueHeader * const header,
    const StoredBinaryValue * const storedBinaryValue,
    const PropertyValueType propertyValueType,
    void * const converterState,
    Exception * const exception) {
#	ifdef _MSC_VER
    UNREFERENCED_PARAMETER(exception);
#	endif

    ((WeightedBool*)header)->value = StoredBinaryValueToBoolOrDefault(
        storedBinaryValue, propertyValueType, *(bool*)converterState);
}

/**
 * @brief Initializes a double weighted value.
 * 
 * Sets the double value from the converter state.
 * 
 * @param header Pointer to the weighted value header
 * @param converterState Pointer to the default double value
 */
static void InitDouble(
    WeightedValueHeader * const header,
    void * const converterState) {
    ((WeightedDouble*)header)->value = *(double*)converterState;
}
/**
 * @brief Saves a double value from a stored binary value.
 * 
 * Converts the stored binary value to a double and saves it in the weighted value.
 * 
 * @param header Pointer to the weighted value header
 * @param storedBinaryValue Pointer to the stored binary value
 * @param propertyValueType Type of the property value
 * @param converterState Pointer to the default double value
 * @param exception Pointer to an exception structure (unused)
 */
static void SaveDouble(
    WeightedValueHeader * const header,
    const StoredBinaryValue * const storedBinaryValue,
    const PropertyValueType propertyValueType,
    void * const converterState,
    Exception * const exception) {
#	ifdef _MSC_VER
    UNREFERENCED_PARAMETER(exception);
#	endif

    ((WeightedDouble*)header)->value = StoredBinaryValueToDoubleOrDefault(
        storedBinaryValue, propertyValueType, *(double*)converterState);
}

/**
 * @brief Initializes a byte weighted value.
 * 
 * Sets the byte value from the converter state.
 * 
 * @param header Pointer to the weighted value header
 * @param converterState Pointer to the default byte value
 */
static void InitByte(
    WeightedValueHeader * const header,
    void * const converterState) {
    ((WeightedByte*)header)->value = *(byte*)converterState;
}
/**
 * @brief Saves a byte value from a stored binary value.
 * 
 * Converts the stored binary value to a byte and saves it in the weighted value.
 * 
 * @param header Pointer to the weighted value header
 * @param storedBinaryValue Pointer to the stored binary value
 * @param propertyValueType Type of the property value
 * @param converterState Pointer to the default byte value
 * @param exception Pointer to an exception structure (unused)
 */
static void SaveByte(
    WeightedValueHeader * const header,
    const StoredBinaryValue * const storedBinaryValue,
    const PropertyValueType propertyValueType,
    void * const converterState,
    Exception * const exception) {
#	ifdef _MSC_VER
    UNREFERENCED_PARAMETER(exception);
#	endif

    ((WeightedByte*)header)->value = (byte)StoredBinaryValueToIntOrDefault(
        storedBinaryValue, propertyValueType, *(uint8_t*)converterState);
}

/**
 * @brief Initializes a string weighted value.
 * 
 * Resets the string data and sets the value pointer to NULL.
 * 
 * @param header Pointer to the weighted value header
 * @param converterState Pointer to converter state (unused)
 */
static void InitString(
    WeightedValueHeader * const header,
    void * const converterState) {
#	ifdef _MSC_VER
    UNREFERENCED_PARAMETER(converterState);
#	endif
    WeightedString * const wString = (WeightedString*)header;
    DataReset(&wString->stringData);
    wString->value = NULL;
}
/**
 * @brief State structure for string value conversion.
 * 
 * Contains information needed for converting values to strings.
 */
typedef struct {
    const uint8_t decimalPlaces;            /**< Number of decimal places for floating-point values */
    Data * const tempData;                  /**< Temporary data structure for string operations */
} StringConverterState;
/**
 * @brief Saves a string value from a stored binary value.
 * 
 * Converts the stored binary value to a string and saves it in the weighted value.
 * This function handles memory allocation for the string.
 * 
 * @param header Pointer to the weighted value header
 * @param storedBinaryValue Pointer to the stored binary value
 * @param propertyValueType Type of the property value
 * @param converterState Pointer to the string converter state
 * @param exception Pointer to an exception structure for error handling
 */
static void SaveString(
    WeightedValueHeader * const header,
    const StoredBinaryValue * const storedBinaryValue,
    const PropertyValueType propertyValueType,
    void * const converterState,
    Exception * const exception) {

    StringConverterState * const state = (StringConverterState *)converterState;
    StringBuilder builder = {
        (char *)state->tempData->ptr,
        state->tempData->allocated,
    };
    StringBuilderInit(&builder);
    StringBuilderAddStringValue(
        &builder,
        storedBinaryValue,
        propertyValueType,
        state->decimalPlaces,
        exception);
    StringBuilderComplete(&builder);
    size_t added = builder.added;
    if (EXCEPTION_OKAY && builder.added > builder.length) {
        DataMalloc(state->tempData, builder.added + 2);
        StringBuilder builder2 = {
            (char *)state->tempData->ptr,
            state->tempData->allocated,
        };
        StringBuilderInit(&builder2);
        StringBuilderAddStringValue(
            &builder2,
            storedBinaryValue,
            propertyValueType,
            state->decimalPlaces,
            exception);
        StringBuilderComplete(&builder2);
        added = builder.added;
    }
    if (EXCEPTION_OKAY) {
        WeightedString * const wString = (WeightedString*)header;
        DataMalloc(&wString->stringData, added + 1);
        memcpy(
            wString->stringData.ptr,
            state->tempData->ptr,
            added + 1);
        wString->value = (char*)wString->stringData.ptr;
    }
}
/**
 * @brief Frees resources used by a string weighted value.
 * 
 * Releases the memory allocated for the string data.
 * 
 * @param header Pointer to the weighted value header
 */
static void FreeString(WeightedValueHeader * const header) {
    WeightedString * const wString = (WeightedString*)header;
    if (wString->stringData.allocated) {
        Free(wString->stringData.ptr);
        DataReset(&wString->stringData);
    }
    wString->value = NULL;
}

static const PropValuesConverter PropValuesConverter_Int = {
    "PropValuesConverter_Int",
    FIFTYONE_DEGREES_PROPERTY_VALUE_TYPE_INTEGER,
    InitInt,
    SaveInt,
    NULL,
    sizeof(WeightedInt),
};
static const PropValuesConverter PropValuesConverter_Double = {
    "PropValuesConverter_Double",
    FIFTYONE_DEGREES_PROPERTY_VALUE_TYPE_DOUBLE,
    InitDouble,
    SaveDouble,
    NULL,
    sizeof(WeightedDouble),
};
static const PropValuesConverter PropValuesConverter_Bool = {
    "PropValuesConverter_Bool",
    FIFTYONE_DEGREES_PROPERTY_VALUE_TYPE_BOOLEAN,
    InitBool,
    SaveBool,
    NULL,
    sizeof(WeightedBool),
};
static const PropValuesConverter PropValuesConverter_Byte = {
    "PropValuesConverter_Byte",
    FIFTYONE_DEGREES_PROPERTY_VALUE_SINGLE_BYTE,
    InitByte,
    SaveByte,
    NULL,
    sizeof(WeightedByte),
};
static const PropValuesConverter PropValuesConverter_String = {
    "PropValuesConverter_String",
    FIFTYONE_DEGREES_PROPERTY_VALUE_TYPE_STRING,
    InitString,
    SaveString,
    FreeString,
    sizeof(WeightedString),
};


/**
 * @brief Gets the appropriate converter for a property value type.
 * 
 * Returns a pointer to the converter structure that can handle the specified value type.
 * 
 * @param valueType The type of property value
 * @return Pointer to the appropriate converter
 */
static const PropValuesConverter * PropValuesConverterFor(
    const PropertyValueType valueType) {
    switch (valueType) {
        case FIFTYONE_DEGREES_PROPERTY_VALUE_TYPE_INTEGER:
            return &PropValuesConverter_Int;
        case FIFTYONE_DEGREES_PROPERTY_VALUE_TYPE_DOUBLE:
            return &PropValuesConverter_Double;
        case FIFTYONE_DEGREES_PROPERTY_VALUE_TYPE_BOOLEAN:
            return &PropValuesConverter_Bool;
        case FIFTYONE_DEGREES_PROPERTY_VALUE_SINGLE_BYTE:
            return &PropValuesConverter_Byte;
        default:
            assert(false);
        case FIFTYONE_DEGREES_PROPERTY_VALUE_TYPE_STRING:
            return &PropValuesConverter_String;
    }
}


/**
 * @brief Context structure for populating a property values chunk.
 * 
 * Contains all the information needed to populate a chunk with values.
 */
typedef struct {
    PropValuesChunk * const chunk;                  /**< Pointer to the chunk to populate */
    const WeightedItem * const valuesItems;         /**< Array of weighted items */
    const uint32_t valuesCount;                     /**< Number of values */
    const PropertyValueType storedValueType;        /**< Type of the stored values */
    Exception * const exception;                    /**< Pointer to exception structure */
} PropValuesChunkContext;

/**
 * @brief Populates a property values chunk with values.
 * 
 * Allocates memory for the chunk data and initializes all values in the chunk.
 * 
 * @param context Pointer to the chunk context
 * @param converter Pointer to the converter for the value type
 * @param converterState Pointer to converter-specific state data
 */
static void PropValuesChunkPopulate(
    const PropValuesChunkContext * const context,
    const PropValuesConverter * const converter,
    void * const converterState) {

    Exception * const exception = context->exception;

    context->chunk->converter = converter;
    DataMalloc(
        &context->chunk->data,
        context->chunk->count * converter->itemSize);
    context->chunk->data.used = context->chunk->data.allocated;
    uint8_t * const chunkDataPtr = context->chunk->data.ptr;

    for (uint32_t i = 0; i < context->valuesCount; i++) {
        WeightedValueHeader * const header = (WeightedValueHeader *)(
            converter->itemSize * i + chunkDataPtr);
        converter->itemInitFunc(header, converterState);
    }
    for (uint32_t i = 0; (i < context->valuesCount) && EXCEPTION_OKAY; i++) {
        WeightedValueHeader * const header = (WeightedValueHeader *)(
            converter->itemSize * i + chunkDataPtr);
        header->requiredPropertyIndex = context->chunk->requiredPropertyIndex;
        header->rawWeighting = context->valuesItems[i].rawWeighting;
        header->valueType = converter->valueType;
        const StoredBinaryValue * const binaryValue = (StoredBinaryValue *)(
            context->valuesItems[i].item.data.ptr);
        converter->itemSaveFunc(
            header,
            binaryValue,
            context->storedValueType,
            converterState,
            exception);
    }
}

/**
 * @brief Populates a property values chunk with integer values.
 * 
 * Wrapper for PropValuesChunkPopulate that uses the integer converter.
 * 
 * @param context Pointer to the chunk context
 * @param defaultValue Default integer value to use
 */
static void PropValuesChunkPopulate_Int(
    const PropValuesChunkContext * const context,
    const int defaultValue) {
    PropValuesChunkPopulate(
        context,
        &PropValuesConverter_Int,
        (void *)&defaultValue);
}
/**
 * @brief Populates a property values chunk with double values.
 * 
 * Wrapper for PropValuesChunkPopulate that uses the double converter.
 * 
 * @param context Pointer to the chunk context
 * @param defaultValue Default double value to use
 */
static void PropValuesChunkPopulate_Double(
    const PropValuesChunkContext * const context,
    const double defaultValue) {
    PropValuesChunkPopulate(
        context,
        &PropValuesConverter_Double,
        (void *)&defaultValue);
}
/**
 * @brief Populates a property values chunk with byte values.
 * 
 * Wrapper for PropValuesChunkPopulate that uses the byte converter.
 * 
 * @param context Pointer to the chunk context
 * @param defaultValue Default byte value to use
 */
static void PropValuesChunkPopulate_Byte(
    const PropValuesChunkContext * const context,
    const byte defaultValue) {
    PropValuesChunkPopulate(
        context,
        &PropValuesConverter_Byte,
        (void *)&defaultValue);
}
/**
 * @brief Populates a property values chunk with boolean values.
 * 
 * Wrapper for PropValuesChunkPopulate that uses the boolean converter.
 * 
 * @param context Pointer to the chunk context
 * @param defaultValue Default boolean value to use
 */
static void PropValuesChunkPopulate_Bool(
    const PropValuesChunkContext * const context,
    const bool defaultValue) {
    PropValuesChunkPopulate(
        context,
        &PropValuesConverter_Bool,
        (void *)&defaultValue);
}
/**
 * @brief Populates a property values chunk with string values.
 * 
 * Wrapper for PropValuesChunkPopulate that uses the string converter.
 * 
 * @param context Pointer to the chunk context
 * @param tempData Temporary data structure for string conversion operations
 * @param decimalPlaces Number of decimal places for floating-point to string conversion
 */
static void PropValuesChunkPopulate_String(
    const PropValuesChunkContext * const context,
    fiftyoneDegreesData * const tempData,
    const uint8_t decimalPlaces) {
    StringConverterState state = {
        decimalPlaces,
        tempData,
    };
    PropValuesChunkPopulate(
        context,
        &PropValuesConverter_String,
        (void *)&state);
}

/**
 * @brief Default values for property value conversion.
 * 
 * Contains default values for each supported property value type.
 */
typedef struct {
    int intValue;                           /**< Default integer value */
    double doubleValue;                     /**< Default double value */
    bool boolValue;                         /**< Default boolean value */
    uint8_t byteValue;                      /**< Default byte value */
    uint8_t stringDecimalPlaces;            /**< Default decimal places for string conversion */
} PropValuesItemConversionDefaults;

/**
 * @brief Initializes a property values chunk with values from results.
 * 
 * Gets the property values from the results and populates the chunk with
 * the appropriate type of values.
 * 
 * @param chunk Pointer to the chunk to initialize
 * @param results Pointer to the IP Intelligence results
 * @param defaults Pointer to the default values for conversion
 * @param tempData Temporary data structure for string conversion operations
 * @param exception Pointer to an exception structure for error handling
 */
static void PropValuesChunkInit(
    PropValuesChunk * const chunk,
    ResultsIpi * const results,
    const PropValuesItemConversionDefaults * const defaults,
    fiftyoneDegreesData * const tempData,
    Exception * const exception) {

    const DataSetIpi * const dataSet = (DataSetIpi*)results->b.dataSet;
    const uint32_t propertyIndex = PropertiesGetPropertyIndexFromRequiredIndex(
        dataSet->b.b.available,
        chunk->requiredPropertyIndex);

    // We should not have any undefined data type in the data file
    // If there is, the data file is not good to use so terminates.
    const PropertyValueType valueType = PropertyGetValueType(
        dataSet->properties, propertyIndex, exception);
    if (EXCEPTION_FAILED) {
        return;
    }

    const PropertyValueType storedValueType = PropertyGetStoredTypeByIndex(
        dataSet->propertyTypes,
        propertyIndex,
        exception);
    if (EXCEPTION_FAILED) {
        return;
    }

    // Get a pointer to the first value item for the property.
    const WeightedItem * const valuesItems = ResultsIpiGetValues(
        results,
        chunk->requiredPropertyIndex,
        exception);
    if (EXCEPTION_FAILED) {
        return;
    }
    if (valuesItems == NULL) {
        return;
    }

    chunk->count = results->values.count;
    const PropValuesChunkContext context = {
        chunk,
        valuesItems,
        chunk->count,
        storedValueType,
        exception,
    };

    switch (valueType) {
        case FIFTYONE_DEGREES_PROPERTY_VALUE_TYPE_INTEGER:
            PropValuesChunkPopulate_Int(&context, defaults->intValue);
            break;
        case FIFTYONE_DEGREES_PROPERTY_VALUE_SINGLE_PRECISION_FLOAT:
        case FIFTYONE_DEGREES_PROPERTY_VALUE_TYPE_DOUBLE:
            PropValuesChunkPopulate_Double(&context, defaults->doubleValue);
            break;
        case FIFTYONE_DEGREES_PROPERTY_VALUE_TYPE_BOOLEAN:
            PropValuesChunkPopulate_Bool(&context, defaults->boolValue);
            break;
        case FIFTYONE_DEGREES_PROPERTY_VALUE_SINGLE_BYTE:
            PropValuesChunkPopulate_Byte(&context, defaults->byteValue);
            break;
        case FIFTYONE_DEGREES_PROPERTY_VALUE_TYPE_STRING:
        default:
            PropValuesChunkPopulate_String(
                &context,
                tempData,
                defaults->stringDecimalPlaces);
            break;
    }
    chunk->data.used = chunk->data.allocated;
}

/**
 * @brief Populates all chunks in a PropValues structure.
 * 
 * Initializes each chunk in the PropValues structure with values from the results.
 * 
 * @param values Pointer to the PropValues structure to populate
 * @param results Pointer to the IP Intelligence results
 * @param defaults Pointer to the default values for conversion
 * @param tempData Temporary data structure for string conversion operations
 * @param exception Pointer to an exception structure for error handling
 */
static void PropValuesPopulate(
    const PropValues * const values,
    ResultsIpi * const results,
    const PropValuesItemConversionDefaults * const defaults,
    fiftyoneDegreesData * const tempData,
    Exception * const exception) {

    for (uint32_t i = 0, n = values->count; (i < n) && EXCEPTION_OKAY; i++) {
        PropValuesChunkInit(
            &(values->items[i]),
            results,
            defaults,
            tempData,
            exception);
    }
}

/**
 * @brief Moves items from PropValues to a WeightedValuesCollection.
 * 
 * Allocates memory in the result collection and copies all values from the
 * PropValues structure to the collection. This function also builds the
 * table of contents for the collection.
 * 
 * @param values Pointer to the PropValues structure containing the values
 * @param result Pointer to the WeightedValuesCollection to populate
 */
static void PropValuesMoveItems(
    const PropValues * const values,
    WeightedValuesCollection * const result) {

    size_t totalSize = 0;
    uint32_t totalCount = 0;
    for (uint32_t i = 0, n = values->count; i < n; i++) {
        totalSize += values->items[i].data.allocated;
        totalCount += values->items[i].count;
    }
    DataMalloc(&result->valuesData, totalSize);
    DataMalloc(
        &result->itemsData,
        totalCount * sizeof(WeightedValueHeader*));
    result->items = (WeightedValueHeader**)result->itemsData.ptr;
    uint8_t *nextChunkNew = (uint8_t *)result->valuesData.ptr;
    for (uint32_t i = 0, n = values->count; i < n; i++) {
        PropValuesChunk * const nextChunk = &(values->items[i]);
        const uint8_t *nextHeaderStart = (const uint8_t *)nextChunkNew;
        memcpy(nextChunkNew, nextChunk->data.ptr, nextChunk->data.allocated);
        result->valuesData.used += nextChunk->data.allocated;
        nextChunkNew += nextChunk->data.allocated;
        if (nextChunk->data.allocated) {
            Free(nextChunk->data.ptr);
            DataReset(&nextChunk->data);
        }
        for (uint32_t j = 0, m = nextChunk->count; j < m; j++) {
            WeightedValueHeader* const nextHeader =
                (WeightedValueHeader*)nextHeaderStart;
            result->items[result->itemsCount] = nextHeader;
            nextHeaderStart += nextChunk->converter->itemSize;
            result->itemsCount++;
        }
    }
}

/* Implementation of the function declared in the header file */
WeightedValuesCollection fiftyoneDegreesResultsIpiGetValuesCollection(
    ResultsIpi * const results,
    const int * const requiredPropertyIndexes,
    const uint32_t requiredPropertyIndexesLength,
    fiftyoneDegreesData * const tempData,
    Exception * const exception) {

    const PropValuesItemConversionDefaults defaults = {
        0,
        0.0,
        false,
        0x00,
        DefaultWktDecimalPlaces,
    };

    WeightedValuesCollection result;
    DataReset(&result.valuesData);
    DataReset(&result.itemsData);
    result.items = NULL;
    result.itemsCount = 0;

    PropValues values;

    const DataSetIpi * const dataSet = (DataSetIpi*)results->b.dataSet;
    if (requiredPropertyIndexes) {
        if (requiredPropertyIndexesLength <= 0) {
            EXCEPTION_SET(INVALID_INPUT);
            return result;
        }
        PropValuesInit(&values, requiredPropertyIndexesLength);
        for (uint32_t i = 0; i < requiredPropertyIndexesLength; i++) {
            values.items[i].requiredPropertyIndex = requiredPropertyIndexes[i];
        }
    } else {
        const uint32_t propsCount = dataSet->b.b.available->count;
        PropValuesInit(&values, propsCount);
        for (uint32_t i = 0; i < propsCount; i++) {
            values.items[i].requiredPropertyIndex = (int)i;
        }
    }
    {
        Data myTempData;
        Data * const theTempData = (tempData
            ? tempData
            : (DataReset(&myTempData), &myTempData));
        PropValuesPopulate(&values, results, &defaults, theTempData, exception);
        if ((theTempData == &myTempData) && myTempData.allocated) {
            Free(myTempData.ptr);
            DataReset(&myTempData);
        }
    }
    PropValuesMoveItems(&values, &result);
    PropValuesRelease(&values);
    if (EXCEPTION_FAILED) {
        fiftyoneDegreesWeightedValuesCollectionRelease(&result);
    }
    return result;
}

/* Implementation of the function declared in the header file */
void fiftyoneDegreesWeightedValuesCollectionRelease(
    WeightedValuesCollection * const collection) {

    if (collection->items && collection->itemsCount > 0) {
        for (uint32_t i = 0, n = collection->itemsCount; i < n; i++) {
            WeightedValueHeader * const nextHeader = collection->items[i];
            const PropertyValueType valueType = nextHeader->valueType;
            const PropValuesConverter * const converter = (
                PropValuesConverterFor(valueType));
            const PropValueFreeFunc freeFunc = converter->itemFreeFunc;
            if (freeFunc) {
                freeFunc(collection->items[i]);
            }
        }
    }
    collection->items = NULL;
    collection->itemsCount = 0;
    if (collection->itemsData.allocated) {
        Free(collection->itemsData.ptr);
        DataReset(&collection->itemsData);
    }
    if (collection->valuesData.allocated) {
        Free(collection->valuesData.ptr);
        DataReset(&collection->valuesData);
    }
}
