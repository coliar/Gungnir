/**
  ******************************************************************************
  * @file    sdmmc.h
  * @brief   This file contains all the function prototypes for
  *          the sdmmc.c file
  ******************************************************************************
  * @attention
  *
  * Copyright (c) 2023 STMicroelectronics.
  * All rights reserved.
  *
  * This software is licensed under terms that can be found in the LICENSE file
  * in the root directory of this software component.
  * If no LICENSE file comes with this software, it is provided AS-IS.
  *
  ******************************************************************************
  */

#ifndef __SDMMC_H__
#define __SDMMC_H__

#ifdef __cplusplus
extern "C" {
#endif

#include "main.h"

#define SDMMC_TEST 0

extern SD_HandleTypeDef SDHandle;


extern int sdmmc_init();

#if SDMMC_TEST

extern int sdmmc_test();

#endif

#ifdef __cplusplus
}
#endif

#endif /* __SDMMC_H__ */

