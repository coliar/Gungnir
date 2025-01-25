#ifndef __SDRAM_H
#define	__SDRAM_H

#include "stm32h7xx.h"

#define KB 1024
#define MB (1024 * KB)

#define W9825G6KH_SIZE 32 * MB 
#define sdram_addr 0xc0000000
#define sdram_size W9825G6KH_SIZE

#define FMC_BANK_SDRAM            FMC_Bank1_SDRAM  
#define FMC_COMMAND_TARGET_BANK   FMC_SDRAM_CMD_TARGET_BANK1
#define SDRAM_BANK_ADDR     ((uint32_t)0xc0000000)
#define SDRAM_MEMORY_WIDTH    FMC_SDRAM_MEM_BUS_WIDTH_16 
#define SDRAM_CAS_LATENCY    FMC_SDRAM_CAS_LATENCY_3
#define SDCLOCK_PERIOD    FMC_SDRAM_CLOCK_PERIOD_2        /* Default configuration used with LCD */
#define SDRAM_READBURST    FMC_SDRAM_RBURST_DISABLE    /* Default configuration used with LCD */
#define SDRAM_TIMEOUT                    ((uint32_t)0xFFFF)


#define SDRAM_MODEREG_BURST_LENGTH_1             ((uint16_t)0x0000)
#define SDRAM_MODEREG_BURST_LENGTH_2             ((uint16_t)0x0001)
#define SDRAM_MODEREG_BURST_LENGTH_4             ((uint16_t)0x0002)
#define SDRAM_MODEREG_BURST_LENGTH_8             ((uint16_t)0x0004)
#define SDRAM_MODEREG_BURST_TYPE_SEQUENTIAL      ((uint16_t)0x0000)
#define SDRAM_MODEREG_BURST_TYPE_INTERLEAVED     ((uint16_t)0x0008)
#define SDRAM_MODEREG_CAS_LATENCY_2              ((uint16_t)0x0020)
#define SDRAM_MODEREG_CAS_LATENCY_3              ((uint16_t)0x0030)
#define SDRAM_MODEREG_OPERATING_MODE_STANDARD    ((uint16_t)0x0000)
#define SDRAM_MODEREG_WRITEBURST_MODE_PROGRAMMED ((uint16_t)0x0000) 
#define SDRAM_MODEREG_WRITEBURST_MODE_SINGLE     ((uint16_t)0x0200)      


extern void 	SDRAM_InitSequence(void);
extern int sdram_test();


#endif /* __SDRAM_H */
