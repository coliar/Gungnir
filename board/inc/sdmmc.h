/**
    ******************************************************************************
    * @file    sdmmc.h
    * @brief   This file contains all the function prototypes for
    *          the sdmmc.c file
    ******************************************************************************
    */

#ifndef __SDMMC_H__
#define __SDMMC_H__

#ifdef __cplusplus
extern "C" {
#endif

#include "board.h"

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

