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

#include "dataset.h"
#include "fiftyone.h"

MAP_TYPE(ConfigBase)
#define CONFIG(d) ((ConfigBase*)d->config)

static StatusCode allocate(
	DataSetBase **replacement, 
	size_t dataSetSize) {
	*replacement = (DataSetBase*)Malloc(dataSetSize);
	return *replacement == NULL ? INSUFFICIENT_MEMORY : SUCCESS;
}

static StatusCode replace(
	ResourceManager *manager,
	DataSetBase *replacement) {

	// Switch the active data set for the new one.
	ResourceReplace(manager, replacement, &replacement->handle);
	if (replacement->handle == NULL) {
		DataSetFree(replacement);
		Free(replacement);
		return INSUFFICIENT_MEMORY;
	}

	return SUCCESS;
}

static StatusCode initWithTempFile(
	DataSetBase *dataSet,
	FileOffset bytesToCompare) {
	if (CONFIG(dataSet)->reuseTempFile == false ||
		FileGetExistingTempFile(
			dataSet->masterFileName,
			CONFIG(dataSet)->tempDirs,
			CONFIG(dataSet)->tempDirCount,
			bytesToCompare,
			dataSet->fileName) == false) {
		return FileNewTempFile(
			dataSet->masterFileName,
			CONFIG(dataSet)->tempDirs,
			CONFIG(dataSet)->tempDirCount,
			(char *)dataSet->fileName,
			sizeof(dataSet->fileName)/sizeof(dataSet->fileName[0]));
	}
	return SUCCESS;
}

void fiftyoneDegreesDataSetFree(fiftyoneDegreesDataSetBase *dataSet) {

	// Free the memory used for the index of property and profile values.
	if (dataSet->indexPropertyProfile != NULL) {
		IndicesPropertyProfileFree(dataSet->indexPropertyProfile);
		dataSet->indexPropertyProfile = NULL;
	}

	// Free the memory used by the unique headers.
	HeadersFree(dataSet->uniqueHeaders);
	dataSet->uniqueHeaders = NULL;

	// Free the override properties if any.
	if (dataSet->overridable != NULL) {
		OverridePropertiesFree(dataSet->overridable);
		dataSet->overridable = NULL;
	}

	// Free the memory used by the available properties.
	PropertiesFree(dataSet->available);
	dataSet->available = NULL;

	// Free the file handles and memory used by the reader.
	FilePoolRelease(&dataSet->filePool);

	// Free memory used to load the file into memory if still requires
	// if used.
	if (dataSet->memoryToFree != NULL) {
		Free(dataSet->memoryToFree);
		dataSet->memoryToFree = NULL;
	}

	// Delete the temp file if one was used.
	if (CONFIG(dataSet)->useTempFile == true) {
		FileDelete(dataSet->fileName);
	}
}

void fiftyoneDegreesDataSetReset(fiftyoneDegreesDataSetBase *dataSet) {
	FilePoolReset(&dataSet->filePool);
	memset((char*)dataSet->fileName, 0, sizeof(dataSet->fileName));
	memset((char*)dataSet->masterFileName, 0, sizeof(dataSet->masterFileName));
	dataSet->memoryToFree = NULL;
	dataSet->isInMemory = false;
	dataSet->uniqueHeaders = NULL;
	dataSet->available = NULL;
	dataSet->overridable = NULL;
	dataSet->indexPropertyProfile = NULL;
	dataSet->config = NULL;
	dataSet->handle = NULL;
}

fiftyoneDegreesStatusCode fiftyoneDegreesDataSetInitProperties(
	fiftyoneDegreesDataSetBase *dataSet,
	fiftyoneDegreesPropertiesRequired *properties,
	void *state,
	fiftyoneDegreesPropertiesGetMethod getPropertyMethod,
	fiftyoneDegreesEvidencePropertiesGetMethod getEvidencePropertiesMethod) {
	uint32_t i;

	// Initialise the available properties.
	dataSet->available = PropertiesCreate(
		properties,
		state,
		getPropertyMethod,
		getEvidencePropertiesMethod);

	// Check the properties were initialised.
	if (dataSet->available == NULL) {
		return REQ_PROP_NOT_PRESENT;
	}

	// Check there are properties available for retrieval.
	if (dataSet->available->count == 0) {
		return REQ_PROP_NOT_PRESENT;
	}

	// Check that all property names were successfully retrieved from the
	// data source.
	for (i = 0; i < dataSet->available->count; i++) {
		if (dataSet->available->items[i].name.data.ptr == NULL) {
			return COLLECTION_FAILURE;
		}
	}

	// Check that all the evidence properties were successfully retrived from
	// the data source.
	for (i = 0; i < dataSet->available->count; i++) {
		if (dataSet->available->items[i].evidenceProperties == NULL) {
			return INSUFFICIENT_MEMORY;
		}
		if (dataSet->available->items[i].evidenceProperties->capacity !=
			dataSet->available->items[i].evidenceProperties->count) {
			return COLLECTION_FAILURE;
		}
	}
	
	return SUCCESS;
}

fiftyoneDegreesStatusCode fiftyoneDegreesDataSetInitHeaders(
	fiftyoneDegreesDataSetBase *dataSet,
	void *state,
	fiftyoneDegreesHeadersGetMethod getHeaderMethod,
	fiftyoneDegreesException* exception) {

	// Initialise the unique HTTP headers.
	dataSet->uniqueHeaders = HeadersCreate(
		CONFIG(dataSet)->usesUpperPrefixedHeaders,
		state,
		getHeaderMethod,
		exception);

	// Check both the headers and properties were initialised.
	if (dataSet->uniqueHeaders == NULL) {
		return CORRUPT_DATA;
	}

	return SUCCESS;
}

fiftyoneDegreesStatusCode fiftyoneDegreesDataSetInitFromFile(
	fiftyoneDegreesDataSetBase *dataSet,
	const char *fileName,
	FileOffset bytesToCompare) {
	char *copiedString;
	// Add 1 for the null terminator
	size_t fileNameLength = strlen(fileName) + 1;

	// Check there is sufficient space to store the filename provided.
	if (fileNameLength > sizeof(dataSet->masterFileName) ||
		fileNameLength > sizeof(dataSet->fileName)) {
		return FILE_PATH_TOO_LONG;
	}

#if defined(__linux__) && __GNUC__ >= 7
#pragma GCC diagnostic push
#pragma GCC diagnostic ignored "-Wstringop-overflow"
// strncpy is called using the length computed on the string length
// adding 1 for null terminator. This is valid and should not cause
// overflow as we have properly checked the buffer size above. Thus
// suppress the warning here.
#endif
	// Use the file name provided as the master data file for the data set.
	copiedString = strncpy(
		(char*)dataSet->masterFileName,
		fileName,
		sizeof(dataSet->masterFileName));
#if defined(__linux__) && __GNUC__ >= 7
#pragma GCC diagnostic pop
#endif
	if (strncmp(fileName, copiedString, fileNameLength) != 0) {
		return CORRUPT_DATA;
	}

	// If temporary files should be used to enable the master data file to
	// be updated during a reload operation create or reuse a temporary
	// file.
	if (CONFIG(dataSet)->useTempFile == true) {
		return initWithTempFile(dataSet, bytesToCompare);
	}

	// Temporary files are not requested so use the master file name
	// as the working file name.
	copiedString = strncpy(
		(char*)dataSet->fileName,
		dataSet->masterFileName,
		fileNameLength);
	if (strncmp(dataSet->masterFileName, copiedString, fileNameLength) != 0) {
		return CORRUPT_DATA;
	}

	return SUCCESS;
}

fiftyoneDegreesStatusCode fiftyoneDegreesDataSetInitInMemory(
	fiftyoneDegreesDataSetBase *dataSet,
	fiftyoneDegreesMemoryReader *reader) {

	// Read the file into memory checking that the operation completed.
	StatusCode status = FileReadToByteArray(dataSet->fileName, reader);
	
	if (status == SUCCESS) {
		// Set the data set so that memory can be freed.
		dataSet->memoryToFree = reader->current;
	}

	return status;
}

fiftyoneDegreesDataSetBase* fiftyoneDegreesDataSetGet(
	fiftyoneDegreesResourceManager *manager) {
	return (DataSetBase*)ResourceHandleIncUse(manager)->resource;
}

void fiftyoneDegreesDataSetRelease(fiftyoneDegreesDataSetBase *dataSet) {
	ResourceHandleDecUse(dataSet->handle);
}

fiftyoneDegreesStatusCode fiftyoneDegreesDataSetReloadManagerFromMemory(
	fiftyoneDegreesResourceManager *manager,
	void *source,
	FileOffset length,
	size_t dataSetSize,
	fiftyoneDegreesDataSetInitFromMemoryMethod initDataSet,
	fiftyoneDegreesException *exception) {
	DataSetBase *replacement = NULL;
	const void *config;
	PropertiesRequired properties = PropertiesDefault;

	// Reference the properties and config from the existing data set in the
	// replacement.
	properties.existing = ((DataSetBase*)manager->active->resource)->available;
	config = ((DataSetBase*)manager->active->resource)->config;

	// Allocate memory for the replacement dataset.
	StatusCode status = allocate(&replacement, dataSetSize);
	if (status != SUCCESS) {
		return status;
	}

	// Set the memory to free pointer to NULL to indicate that when this
	// new data set is released the memory shouldn't be freed by 51Degrees but
	// by the consumer of the service.
	replacement->memoryToFree = NULL;

	// Initialise the new data set with the data pointed to by source.
	status = initDataSet(
		replacement,
		config,
		&properties,
		source,
		length,
		exception);
	if (status != SUCCESS) {
		Free(replacement);
		return status;
	}
	
	return replace(manager, replacement);
}

fiftyoneDegreesStatusCode fiftyoneDegreesDataSetReloadManagerFromFile(
	fiftyoneDegreesResourceManager* manager,
	const char *fileName,
	size_t dataSetSize,
	fiftyoneDegreesDataSetInitFromFileMethod initDataSet,
	fiftyoneDegreesException *exception) {
	DataSetBase *replacement = NULL;
	const void *config;
	PropertiesRequired properties = PropertiesDefault;

	// Reference the properties and config from the existing data set in the
	// replacement.
	properties.existing = ((DataSetBase*)manager->active->resource)->available;
	config = ((DataSetBase*)manager->active->resource)->config;
	
	// Allocate memory for the replacement dataset.
	StatusCode status = allocate(&replacement, dataSetSize);
	if (status != SUCCESS) {
		return status; 
	}

	// Initialise the new data set with the properties of the current one.
	status = initDataSet(
		replacement,
		config,
		&properties,
		fileName,
		exception);
	if (status != SUCCESS) {
		return status;
	}
	
	return replace(manager, replacement);
}