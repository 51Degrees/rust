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

#ifndef FIFTYONE_DEGREES_CONFIG_H_INCLUDED
#define FIFTYONE_DEGREES_CONFIG_H_INCLUDED

/**
 * @ingroup FiftyOneDegreesCommon
 * @defgroup FiftyOneDegreesConfig Config
 *
 * Configuration for building data sets.
 *
 * ## Introduction
 *
 * Configuration structures based off the base configuration type are used when
 * building data sets. The base configuration describes how the data is handled.
 * For example, whether or not a temporary file should be created, or whether
 * the memory should be freed with the data set.
 *
 * Extending configurations will add options specific to certain categories of
 * data sets.
 *
 * @{
 */

/**
 * Base configuration structure containing common configuration options, and
 * options that apply to structures and methods in the common library.
 */
typedef struct fiftyone_degrees_config_base_t {
	bool allInMemory; /**< True if the data file should be loaded entirely into
	                      continuous memory. */
	bool usesUpperPrefixedHeaders; /**< True if the HTTP header field names
	                                   MIGHT include the prefix HTTP_ */
	bool freeData; /**< True if the memory containing the data set should be
	                   freed after it is no longer needed. This only applies to
	                   externally allocated memory, anything allocated
	                   internally is automatically freed. */
	bool useTempFile; /**< Indicates whether a temporary file should be created
	                      instead of using the original file. */
	bool reuseTempFile; /**< Indicates that an existing temp file may be used.
	                        This should be selected if multiple instances wish
	                        to use the same file to prevent high disk usage. */
	const char **tempDirs; /**< Array of temp directories which can be used in
	                           order of preference. */
	int tempDirCount; /**< Number of directories in the tempDirs array. */
	bool propertyValueIndex; /**< Indicates if an index to values for property 
							     and profiles should be created. */
} fiftyoneDegreesConfigBase;

/** Default value for the #FIFTYONE_DEGREES_CONFIG_USE_TEMP_FILE macro. */
#define FIFTYONE_DEGREES_CONFIG_USE_TEMP_FILE_DEFAULT false

/**
 * Temp file setting used in the default configuration macro
 * #FIFTYONE_DEGREES_CONFIG_DEFAULT.
 */
#define FIFTYONE_DEGREES_CONFIG_USE_TEMP_FILE \
FIFTYONE_DEGREES_CONFIG_USE_TEMP_FILE_DEFAULT

/** Default value for the #FIFTYONE_DEGREES_CONFIG_ALL_IN_MEMORY macro. */
#ifndef FIFTYONE_DEGREES_MEMORY_ONLY
#define FIFTYONE_DEGREES_CONFIG_ALL_IN_MEMORY_DEFAULT false
#else
#define FIFTYONE_DEGREES_CONFIG_ALL_IN_MEMORY_DEFAULT true
#endif

/**
 * All in memory setting used in the default configuration macro
 * #FIFTYONE_DEGREES_CONFIG_DEFAULT.
 */
#define FIFTYONE_DEGREES_CONFIG_ALL_IN_MEMORY \
FIFTYONE_DEGREES_CONFIG_ALL_IN_MEMORY_DEFAULT

/**
 * Default value for the #fiftyoneDegreesConfigBase structure with index.
 */
#define FIFTYONE_DEGREES_CONFIG_DEFAULT_WITH_INDEX \
	FIFTYONE_DEGREES_CONFIG_ALL_IN_MEMORY, /* allInMemory */ \
	true, /* usesUpperPrefixedHeaders */ \
	false, /* freeData */ \
	FIFTYONE_DEGREES_CONFIG_USE_TEMP_FILE, /* useTempFile */ \
	false, /* reuseTempFile */ \
	NULL, /* tempDirs */ \
	0, /* tempDirCount */ \
	true /* propertyValueIndex */

 /**
  * Default value for the #fiftyoneDegreesConfigBase structure without index.
  */
#define FIFTYONE_DEGREES_CONFIG_DEFAULT_NO_INDEX \
	FIFTYONE_DEGREES_CONFIG_ALL_IN_MEMORY, /* allInMemory */ \
	true, /* usesUpperPrefixedHeaders */ \
	false, /* freeData */ \
	FIFTYONE_DEGREES_CONFIG_USE_TEMP_FILE, /* useTempFile */ \
	false, /* reuseTempFile */ \
	NULL, /* tempDirs */ \
	0, /* tempDirCount */ \
	false /* propertyValueIndex */

/**
 * @}
 */
#endif